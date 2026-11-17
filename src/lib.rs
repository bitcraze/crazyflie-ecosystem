#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod error;
pub mod lighthouse;

pub use crazyflie_lib;

pub use crate::error::{Error, Result};
