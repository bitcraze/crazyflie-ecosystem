//! Example demonstrating reading and writing the Lighthouse configuration
//!
//! This example:
//! - `read`: reads base station geometry and calibration data from the
//!   Crazyflie and prints it as YAML
//! - `write <file>`: loads a YAML file, writes geometry and calibration data
//!   to the Crazyflie and persists it to permanent storage
//!
//! The YAML file uses the same format as the Python cflib and cfclient.
//!
//! Usage:
//!   cargo run --example lighthouse_config -- read
//!   cargo run --example lighthouse_config -- write lighthouse.yaml
//!
//! REQUIREMENTS:
//! - Crazyflie with Lighthouse deck

use crazyflie_ecosystem::crazyflie_lib::{Crazyflie, NoTocCache, crazyflie_link::LinkContext};
use crazyflie_ecosystem::lighthouse::{LighthouseConfig, LighthouseSlotReport, read_config, write_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let path = match args.get(1..).unwrap_or_default() {
        [command] if command == "read" => None,
        [command, path] if command == "write" => Some(path.clone()),
        _ => {
            eprintln!("Usage: {} read | write <file.yaml>", args[0]);
            std::process::exit(1);
        }
    };

    let link_context = LinkContext::new();

    // Connect to Crazyflie
    let uri = std::env::var("CFURI").unwrap_or_else(|_| "radio://0/80/2M/E7E7E7E7E7".to_string());
    println!("Connecting to {} ...", uri);

    let crazyflie = Crazyflie::connect_from_uri(&link_context, &uri, NoTocCache).await?;
    println!("Connected!");

    let result = match path {
        None => read(&crazyflie).await,
        Some(path) => write(&crazyflie, &path).await,
    };

    crazyflie.disconnect().await;

    result
}

/// Read the configuration from the Crazyflie and print it as YAML
async fn read(crazyflie: &Crazyflie) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nReading geometries and calibrations...");
    let config = read_config(crazyflie).await?;

    println!("\n{}", config.to_yaml()?);

    Ok(())
}

/// Load a YAML file and write the configuration to the Crazyflie
async fn write(crazyflie: &Crazyflie, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = LighthouseConfig::from_yaml(&std::fs::read_to_string(path)?)?;
    println!("\nLoaded {}", path);

    println!("Setting system type to {}, writing and persisting...", config.system_type);
    let report = write_config(crazyflie, &config).await?;

    println!("Geometries:");
    print_report(&report.geometries);
    println!("Calibrations:");
    print_report(&report.calibrations);

    println!("✓ Configuration written and persisted!");
    Ok(())
}

fn print_report(report: &LighthouseSlotReport) {
    println!("  Written from file: {:?}", report.written);
    println!("  Cleared: {:?}", report.cleared);
    if !report.rejected.is_empty() {
        println!(
            "  Warning: base stations {:?} from the file are not supported by the Crazyflie",
            report.rejected
        );
    }
}
