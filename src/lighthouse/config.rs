//! Lighthouse system configuration file, in the YAML format used by cflib, cfclient and cfcli

use crate::{Error, Result};
use crazyflie_lib::subsystems::memory::{
    LighthouseBsCalibration, LighthouseBsGeometry, LighthouseCalibrationSweep, LighthouseMemory,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

const FILE_TYPE: &str = "lighthouse_system_configuration";
const FILE_VERSION: &str = "1";

/// System type for Lighthouse V1 base stations
pub const LIGHTHOUSE_SYSTEM_TYPE_V1: u8 = 1;
/// System type for Lighthouse V2 base stations
pub const LIGHTHOUSE_SYSTEM_TYPE_V2: u8 = 2;

/// A lighthouse system configuration
///
/// Load and save it with [`from_yaml`](Self::from_yaml) and [`to_yaml`](Self::to_yaml), and
/// read or write it on a Crazyflie with [`read_config`](super::read_config) and
/// [`write_config`](super::write_config).
#[derive(Debug, Clone, PartialEq)]
pub struct LighthouseConfig {
    /// Lighthouse system type ([`LIGHTHOUSE_SYSTEM_TYPE_V1`] or [`LIGHTHOUSE_SYSTEM_TYPE_V2`])
    pub system_type: u8,
    /// Geometry data, mapping base station ID to geometry
    pub geometries: HashMap<u8, LighthouseBsGeometry>,
    /// Calibration data, mapping base station ID to calibration
    pub calibrations: HashMap<u8, LighthouseBsCalibration>,
}

impl LighthouseConfig {
    /// Parse and check a lighthouse configuration from YAML
    ///
    /// `systemType` defaults to [`LIGHTHOUSE_SYSTEM_TYPE_V2`]. Returns [`Error::ConfigFileError`]
    /// if the file is malformed, has an unsupported type, version or system type, or a base
    /// station ID above 15.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let file: ConfigFile = serde_yaml_ng::from_str(yaml).map_err(|e| invalid(&e.to_string()))?;

        match file.file_type.as_deref() {
            None => return Err(invalid("type field missing")),
            Some(FILE_TYPE) => {}
            Some(other) => return Err(invalid(&format!("unsupported file type '{}'", other))),
        }
        match file.version.as_deref() {
            None => return Err(invalid("version field missing")),
            Some(FILE_VERSION) => {}
            Some(other) => return Err(invalid(&format!("unsupported file version '{}'", other))),
        }

        let system_type = file.system_type.unwrap_or(LIGHTHOUSE_SYSTEM_TYPE_V2);
        if system_type != LIGHTHOUSE_SYSTEM_TYPE_V1 && system_type != LIGHTHOUSE_SYSTEM_TYPE_V2 {
            return Err(invalid(&format!("unsupported system type {}", system_type)));
        }

        for &bs_id in file.geos.keys().chain(file.calibs.keys()) {
            if bs_id as usize >= LighthouseMemory::MAX_BASE_STATIONS {
                return Err(invalid(&format!(
                    "base station ID {} out of range (0-{})",
                    bs_id,
                    LighthouseMemory::MAX_BASE_STATIONS - 1
                )));
            }
        }

        let geometries = file
            .geos
            .iter()
            .map(|(&bs_id, geo)| {
                let geometry = LighthouseBsGeometry {
                    origin: geo.origin,
                    rotation_matrix: geo.rotation,
                    valid: true,
                };
                (bs_id, geometry)
            })
            .collect();
        let calibrations = file
            .calibs
            .iter()
            .map(|(&bs_id, calib)| {
                let calibration = LighthouseBsCalibration {
                    sweeps: [(&calib.sweeps[0]).into(), (&calib.sweeps[1]).into()],
                    uid: calib.uid,
                    valid: true,
                };
                (bs_id, calibration)
            })
            .collect();

        Ok(Self { system_type, geometries, calibrations })
    }

    /// Serialize the configuration to YAML
    ///
    /// Invalid geometries and calibrations are left out.
    pub fn to_yaml(&self) -> Result<String> {
        let file = ConfigFile {
            calibs: self
                .calibrations
                .iter()
                .filter(|(_, calib)| calib.valid)
                .map(|(&bs_id, calib)| {
                    let entry = CalibrationEntry {
                        sweeps: [(&calib.sweeps[0]).into(), (&calib.sweeps[1]).into()],
                        uid: calib.uid,
                    };
                    (bs_id, entry)
                })
                .collect(),
            geos: self
                .geometries
                .iter()
                .filter(|(_, geo)| geo.valid)
                .map(|(&bs_id, geo)| {
                    let entry = GeometryEntry {
                        origin: geo.origin,
                        rotation: geo.rotation_matrix,
                    };
                    (bs_id, entry)
                })
                .collect(),
            system_type: Some(self.system_type),
            file_type: Some(FILE_TYPE.to_owned()),
            version: Some(FILE_VERSION.to_owned()),
        };

        serde_yaml_ng::to_string(&file)
            .map_err(|e| Error::ConfigFileError(format!("could not serialize lighthouse config: {}", e)))
    }
}

fn invalid(reason: &str) -> Error {
    Error::ConfigFileError(format!("invalid lighthouse config file: {}", reason))
}

/// The file layout. Fields are in alphabetical order, the same order cflib writes them.
#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    calibs: BTreeMap<u8, CalibrationEntry>,
    #[serde(default)]
    geos: BTreeMap<u8, GeometryEntry>,
    #[serde(rename = "systemType")]
    system_type: Option<u8>,
    #[serde(rename = "type")]
    file_type: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeometryEntry {
    origin: [f32; 3],
    rotation: [[f32; 3]; 3],
}

#[derive(Debug, Serialize, Deserialize)]
struct CalibrationEntry {
    sweeps: [SweepEntry; 2],
    uid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SweepEntry {
    curve: f32,
    gibmag: f32,
    gibphase: f32,
    ogeemag: f32,
    ogeephase: f32,
    phase: f32,
    tilt: f32,
}

impl From<&LighthouseCalibrationSweep> for SweepEntry {
    fn from(sweep: &LighthouseCalibrationSweep) -> Self {
        Self {
            curve: sweep.curve,
            gibmag: sweep.gibmag,
            gibphase: sweep.gibphase,
            ogeemag: sweep.ogeemag,
            ogeephase: sweep.ogeephase,
            phase: sweep.phase,
            tilt: sweep.tilt,
        }
    }
}

impl From<&SweepEntry> for LighthouseCalibrationSweep {
    fn from(entry: &SweepEntry) -> Self {
        Self {
            phase: entry.phase,
            tilt: entry.tilt,
            curve: entry.curve,
            gibmag: entry.gibmag,
            gibphase: entry.gibphase,
            ogeemag: entry.ogeemag,
            ogeephase: entry.ogeephase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as written by cflib (`LighthouseConfigFileManager.write`)
    const CFLIB_FILE: &str = "\
calibs:
  1:
    sweeps:
    - curve: 3.0
      gibmag: 4.0
      gibphase: 5.0
      ogeemag: 6.0
      ogeephase: 7.0
      phase: 1.0
      tilt: 2.0
    - curve: 3.1
      gibmag: 4.1
      gibphase: 5.1
      ogeemag: 6.1
      ogeephase: 7.1
      phase: 1.1
      tilt: 2.1
    uid: 1234
geos:
  1:
    origin:
    - 1.0
    - 2.0
    - 3.0
    rotation:
    - - 4.0
      - 5.0
      - 6.0
    - - 7.0
      - 8.0
      - 9.0
    - - 10.0
      - 11.0
      - 12.0
systemType: 1
type: lighthouse_system_configuration
version: '1'
";

    fn assert_rejected(yaml: &str, reason: &str) {
        match LighthouseConfig::from_yaml(yaml) {
            Err(Error::ConfigFileError(msg)) => {
                assert!(msg.contains(reason), "expected '{}' in '{}'", reason, msg)
            }
            other => panic!("expected ConfigFileError containing '{}', got {:?}", reason, other),
        }
    }

    #[test]
    fn reads_cflib_file() {
        let config = LighthouseConfig::from_yaml(CFLIB_FILE).unwrap();

        assert_eq!(config.system_type, LIGHTHOUSE_SYSTEM_TYPE_V1);

        let geo = &config.geometries[&1];
        assert_eq!(geo.origin, [1.0, 2.0, 3.0]);
        assert_eq!(geo.rotation_matrix, [[4.0, 5.0, 6.0], [7.0, 8.0, 9.0], [10.0, 11.0, 12.0]]);
        assert!(geo.valid);

        let calib = &config.calibrations[&1];
        assert_eq!(calib.uid, 1234);
        assert_eq!(calib.sweeps[0].phase, 1.0);
        assert_eq!(calib.sweeps[1].ogeephase, 7.1);
        assert!(calib.valid);
    }

    #[test]
    fn round_trip_matches_cflib_output() {
        let config = LighthouseConfig::from_yaml(CFLIB_FILE).unwrap();
        let yaml = config.to_yaml().unwrap();
        assert_eq!(LighthouseConfig::from_yaml(&yaml).unwrap(), config);
        assert_eq!(yaml, CFLIB_FILE);
    }

    #[test]
    fn invalid_entries_are_not_saved() {
        let mut config = LighthouseConfig::from_yaml(CFLIB_FILE).unwrap();
        config.geometries.get_mut(&1).unwrap().valid = false;
        config.calibrations.get_mut(&1).unwrap().valid = false;

        let reloaded = LighthouseConfig::from_yaml(&config.to_yaml().unwrap()).unwrap();
        assert!(reloaded.geometries.is_empty());
        assert!(reloaded.calibrations.is_empty());
    }

    #[test]
    fn optional_fields_default() {
        let config =
            LighthouseConfig::from_yaml("type: lighthouse_system_configuration\nversion: '1'\n").unwrap();
        assert_eq!(config.system_type, LIGHTHOUSE_SYSTEM_TYPE_V2);
        assert!(config.geometries.is_empty());
        assert!(config.calibrations.is_empty());
    }

    #[test]
    fn rejects_bad_type() {
        assert_rejected("version: '1'\n", "type field missing");
        assert_rejected("type: something_else\nversion: '1'\n", "unsupported file type");
    }

    #[test]
    fn rejects_bad_version() {
        assert_rejected("type: lighthouse_system_configuration\n", "version field missing");
        assert_rejected("type: lighthouse_system_configuration\nversion: '2'\n", "unsupported file version");
    }

    #[test]
    fn rejects_bad_system_type() {
        assert_rejected(
            "type: lighthouse_system_configuration\nversion: '1'\nsystemType: 3\n",
            "unsupported system type 3",
        );
    }

    #[test]
    fn rejects_base_station_id_out_of_range() {
        let yaml = CFLIB_FILE.replacen("geos:\n  1:", "geos:\n  16:", 1);
        assert_rejected(&yaml, "base station ID 16 out of range");
    }

    #[test]
    fn rejects_malformed_entries() {
        // Geometry with only two origin coordinates
        let yaml = CFLIB_FILE.replacen("    - 3.0\n    rotation", "    rotation", 1);
        assert_rejected(&yaml, "invalid lighthouse config file");
        assert_rejected("not: [valid", "invalid lighthouse config file");
    }
}
