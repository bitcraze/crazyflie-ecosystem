//! # Crazyflie ecosystem
//!
//! Higher-level utilities for the Crazyflie, built on top of [crazyflie-lib].
//!
//! [crazyflie-lib] implements the Crazyflie protocol. Features built on top of it that clients need
//! live here instead of being re-implemented in every client (cfclient, cfcli, scripts, ...). The
//! Python bindings (`cflib2`, [crazyflie-lib-python-v2]) depend on this crate, so every feature here
//! is also available to Python clients.
//!
//! ```text
//! crazyflie-link  ->  crazyflie-lib  ->  crazyflie-ecosystem  ->  cflib2  ->  Python clients
//!                                                 \
//!                                                  ->  Rust clients (cfcli, ...)
//! ```
//!
//! ## Architecture
//!
//! See the README for the full rules. In short:
//!
//! - **crazyflie-lib** is the protocol: the [`Crazyflie`](crazyflie_lib::Crazyflie) type,
//!   subsystems, packet and memory formats.
//! - **crazyflie-ecosystem** is features built from crazyflie-lib calls (or files and math) that more
//!   than one client needs. It never defines its own `Crazyflie`: features are functions that take
//!   `&crazyflie_lib::Crazyflie`. [`crazyflie_lib`] is re-exported and its cargo features are
//!   forwarded, so users of this crate never need a direct crazyflie-lib dependency.
//! - **cflib2** depends only on this crate and is the only place that shapes the Python API.
//!
//! [crazyflie-lib]: https://github.com/bitcraze/crazyflie-lib-rs
//! [crazyflie-lib-python-v2]: https://github.com/bitcraze/crazyflie-lib-python-v2

#![warn(missing_docs)]

mod error;

pub use crazyflie_lib;

pub use crate::error::{Error, Result};
