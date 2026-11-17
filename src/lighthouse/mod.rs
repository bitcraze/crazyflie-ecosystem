//! # Lighthouse
//!
//! Lighthouse positioning utilities built on the lighthouse memory and localization
//! subsystems of [crazyflie-lib](crazyflie_lib).
//!
//! - [`LighthouseConfig`]: load, check and save a lighthouse system configuration file
//! - [`read_config`]: read the configuration from a Crazyflie
//! - [`write_config`]: write a configuration to a Crazyflie and persist it

mod config;
mod crazyflie;

pub use config::*;
pub use crazyflie::*;
