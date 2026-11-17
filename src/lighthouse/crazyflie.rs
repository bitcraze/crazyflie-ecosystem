//! Read and write a lighthouse system configuration on a Crazyflie

use crate::lighthouse::LighthouseConfig;
use crate::{Error, Result};
use crazyflie_lib::Crazyflie;
use crazyflie_lib::subsystems::memory::{
    LighthouseBsCalibration, LighthouseBsGeometry, LighthouseMemory, MemoryDevice, MemoryType,
};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

const SYSTEM_TYPE_PARAM: &str = "lighthouse.systemType";

/// Changing the system type can take up to 0.5 s, and the parameter gives no signal when it is done
const SYSTEM_TYPE_SWITCH_DELAY: Duration = Duration::from_millis(800);

/// Outcome of writing geometry or calibration to all base station slots
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LighthouseSlotReport {
    /// Base stations from the configuration that were written
    pub written: Vec<u8>,
    /// Slots not in the configuration that were cleared
    pub cleared: Vec<u8>,
    /// Base stations from the configuration that the Crazyflie does not support
    pub rejected: Vec<u8>,
}

/// Result of [`write_config`]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LighthouseConfigWriteReport {
    /// Geometry slots
    pub geometries: LighthouseSlotReport,
    /// Calibration slots
    pub calibrations: LighthouseSlotReport,
}

impl LighthouseConfigWriteReport {
    /// True if every base station in the configuration was written
    pub fn is_complete(&self) -> bool {
        self.geometries.rejected.is_empty() && self.calibrations.rejected.is_empty()
    }
}

/// Read the lighthouse system configuration from the Crazyflie
///
/// Only slots with valid data are included.
pub async fn read_config(cf: &Crazyflie) -> Result<LighthouseConfig> {
    let memory = open_memory(cf, find_memory(cf)?).await?;
    let result = read_memory(&memory).await;
    cf.memory.close_memory(memory).await?;
    let (geometries, calibrations) = result?;

    let system_type: u8 = cf.param.get(SYSTEM_TYPE_PARAM).await?;

    Ok(LighthouseConfig { system_type, geometries, calibrations })
}

/// Write a lighthouse system configuration to the Crazyflie and persist it
///
/// Sets the system type, writes all 16 slots (clearing the ones not in `config`) and persists
/// the written slots. Base stations the Crazyflie does not support are skipped and listed in
/// [`LighthouseSlotReport::rejected`].
pub async fn write_config(cf: &Crazyflie, config: &LighthouseConfig) -> Result<LighthouseConfigWriteReport> {
    let device = find_memory(cf)?;

    cf.param.set(SYSTEM_TYPE_PARAM, config.system_type).await?;
    tokio::time::sleep(SYSTEM_TYPE_SWITCH_DELAY).await;

    let memory = open_memory(cf, device).await?;
    let result = write_memory(&memory, config).await;
    cf.memory.close_memory(memory).await?;
    let report = result?;

    let geometries = persisted_slots(&report.geometries);
    let calibrations = persisted_slots(&report.calibrations);
    if !cf.localization.lighthouse.persist_lighthouse_data(&geometries, &calibrations).await? {
        return Err(Error::LighthouseError("the Crazyflie could not persist the configuration".into()));
    }

    Ok(report)
}

fn find_memory(cf: &Crazyflie) -> Result<MemoryDevice> {
    cf.memory
        .get_memories(Some(MemoryType::Lighthouse))
        .first()
        .map(|device| (*device).clone())
        .ok_or_else(|| Error::LighthouseError("no lighthouse memory found, is a Lighthouse deck attached?".into()))
}

async fn open_memory(cf: &Crazyflie, device: MemoryDevice) -> Result<LighthouseMemory> {
    match cf.memory.open_memory::<LighthouseMemory>(device).await {
        Some(memory) => Ok(memory?),
        None => Err(Error::LighthouseError("lighthouse memory is already open".into())),
    }
}

type Geometries = HashMap<u8, LighthouseBsGeometry>;
type Calibrations = HashMap<u8, LighthouseBsCalibration>;

async fn read_memory(memory: &LighthouseMemory) -> Result<(Geometries, Calibrations)> {
    let geometries = read_slots(
        |bs_id| async move { memory.read_geometry(bs_id).await },
        |geometry| geometry.valid,
    )
    .await?;
    let calibrations = read_slots(
        |bs_id| async move { memory.read_calibration(bs_id).await },
        |calibration| calibration.valid,
    )
    .await?;

    Ok((geometries, calibrations))
}

/// Read all slots and keep the valid ones
async fn read_slots<T, F, Fut>(read: F, is_valid: fn(&T) -> bool) -> Result<HashMap<u8, T>>
where
    F: Fn(u8) -> Fut,
    Fut: Future<Output = crazyflie_lib::Result<T>>,
{
    let mut slots = HashMap::new();

    for bs_id in 0..LighthouseMemory::MAX_BASE_STATIONS as u8 {
        match read(bs_id).await {
            Ok(data) if is_valid(&data) => {
                slots.insert(bs_id, data);
            }
            Ok(_) => {}
            // Slot not supported by the Crazyflie
            Err(crazyflie_lib::Error::MemoryError(_)) => {}
            Err(e) => return Err(e.into()),
        }
    }

    Ok(slots)
}

async fn write_memory(memory: &LighthouseMemory, config: &LighthouseConfig) -> Result<LighthouseConfigWriteReport> {
    let geometries = write_slots(&config.geometries, LighthouseBsGeometry::default(), |bs_id, geometry| async move {
        memory.write_geometry(bs_id, &geometry).await
    })
    .await?;
    let calibrations = write_slots(&config.calibrations, LighthouseBsCalibration::default(), |bs_id, calibration| async move {
        memory.write_calibration(bs_id, &calibration).await
    })
    .await?;

    Ok(LighthouseConfigWriteReport { geometries, calibrations })
}

/// Write all slots in ascending order: the configured data, or `empty` for slots not in `configured`
async fn write_slots<T, F, Fut>(configured: &HashMap<u8, T>, empty: T, write: F) -> Result<LighthouseSlotReport>
where
    T: Clone,
    F: Fn(u8, T) -> Fut,
    Fut: Future<Output = crazyflie_lib::Result<()>>,
{
    let mut report = LighthouseSlotReport::default();

    for bs_id in 0..LighthouseMemory::MAX_BASE_STATIONS as u8 {
        let data = configured.get(&bs_id);
        let result = write(bs_id, data.cloned().unwrap_or_else(|| empty.clone())).await;

        match (result, data.is_some()) {
            (Ok(()), true) => report.written.push(bs_id),
            (Ok(()), false) => report.cleared.push(bs_id),
            // Slot not supported by the Crazyflie
            (Err(crazyflie_lib::Error::MemoryError(_)), true) => report.rejected.push(bs_id),
            (Err(crazyflie_lib::Error::MemoryError(_)), false) => {}
            (Err(e), _) => return Err(e.into()),
        }
    }

    Ok(report)
}

fn persisted_slots(report: &LighthouseSlotReport) -> Vec<u8> {
    let mut slots: Vec<u8> = report.written.iter().chain(&report.cleared).copied().collect();
    slots.sort_unstable();
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(future)
    }

    /// A fake Crazyflie supporting `supported` base stations, disconnecting at slot `fail_at`
    fn fake_answer(bs_id: u8, supported: u8, fail_at: Option<u8>) -> crazyflie_lib::Result<()> {
        if Some(bs_id) == fail_at {
            Err(crazyflie_lib::Error::Disconnected)
        } else if bs_id >= supported {
            Err(crazyflie_lib::Error::MemoryError("error status".into()))
        } else {
            Ok(())
        }
    }

    /// Returns the report and every write as (slot, configured)
    fn write_to_fake(
        configured: &[u8],
        supported: u8,
        fail_at: Option<u8>,
    ) -> (Result<LighthouseSlotReport>, Vec<(u8, bool)>) {
        let configured: HashMap<u8, bool> = configured.iter().map(|&bs_id| (bs_id, true)).collect();
        let writes = RefCell::new(Vec::new());

        let result = block_on(write_slots(&configured, false, |bs_id, data| {
            writes.borrow_mut().push((bs_id, data));
            async move { fake_answer(bs_id, supported, fail_at) }
        }));

        (result, writes.into_inner())
    }

    fn read_from_fake(valid: &[u8], supported: u8, fail_at: Option<u8>) -> Result<HashMap<u8, bool>> {
        block_on(read_slots(
            |bs_id| async move { fake_answer(bs_id, supported, fail_at).map(|()| valid.contains(&bs_id)) },
            |&is_valid| is_valid,
        ))
    }

    #[test]
    fn writes_all_slots_in_order_and_clears_unconfigured() {
        let (report, writes) = write_to_fake(&[0, 2], 16, None);
        let report = report.unwrap();

        assert_eq!(writes.iter().map(|&(bs_id, _)| bs_id).collect::<Vec<_>>(), (0..16).collect::<Vec<_>>());
        assert!(writes[0].1 && !writes[1].1 && writes[2].1);
        assert_eq!(report.written, vec![0, 2]);
        assert_eq!(report.cleared, vec![1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        assert!(report.rejected.is_empty());
    }

    #[test]
    fn unsupported_slots_are_skipped() {
        let (report, _) = write_to_fake(&[0, 1, 5, 6], 4, None);
        let report = report.unwrap();

        assert_eq!(report.written, vec![0, 1]);
        assert_eq!(report.cleared, vec![2, 3]);
        assert_eq!(report.rejected, vec![5, 6]);
        assert_eq!(persisted_slots(&report), vec![0, 1, 2, 3]);
    }

    #[test]
    fn other_errors_stop_the_write() {
        let (report, writes) = write_to_fake(&[0, 1], 16, Some(3));

        assert!(matches!(report, Err(Error::LibError(crazyflie_lib::Error::Disconnected))));
        assert_eq!(writes.len(), 4);
    }

    #[test]
    fn report_completeness() {
        let mut report = LighthouseConfigWriteReport::default();
        assert!(report.is_complete());
        report.calibrations.rejected.push(5);
        assert!(!report.is_complete());
    }

    #[test]
    fn reads_valid_slots_and_skips_unsupported() {
        let slots = read_from_fake(&[0, 2, 6], 4, None).unwrap();

        let mut bs_ids: Vec<u8> = slots.keys().copied().collect();
        bs_ids.sort_unstable();
        assert_eq!(bs_ids, vec![0, 2]);
    }

    #[test]
    fn other_errors_stop_the_read() {
        let result = read_from_fake(&[0, 1], 16, Some(1));
        assert!(matches!(result, Err(Error::LibError(crazyflie_lib::Error::Disconnected))));
    }
}
