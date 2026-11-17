# Crazyflie ecosystem

Rust crate with higher-level utilities for the [Crazyflie], built on top of [crazyflie-lib].

[crazyflie-lib] implements the Crazyflie protocol. Features built on top of it that clients need, such
as uploading a lighthouse configuration, live here instead of being re-implemented in every client
(cfclient, cfcli, user scripts, ...). The Python bindings ([cflib2]) depend on this crate, so
everything here is also available from Python.

```text
crazyflie-link  ->  crazyflie-lib  ->  crazyflie-ecosystem  ->  cflib2 (Python bindings)  ->  Python clients
                                                \
                                                 ->  Rust clients (cfcli, ...)
```

## Architecture

- **crazyflie-lib** is the protocol: the `Crazyflie` type, subsystems, packet and memory formats.
  One operation per function. It never depends on crazyflie-ecosystem.
- **crazyflie-ecosystem** is features built from crazyflie-lib calls (or files and math) that more
  than one client needs. It never defines its own `Crazyflie` or wraps crazyflie-lib types: features
  are functions that take `&crazyflie_lib::Crazyflie`. It re-exports `crazyflie_lib` and forwards its
  cargo features, so its users never need a direct crazyflie-lib dependency.
- **cflib2** depends only on crazyflie-ecosystem and is the only place that shapes the Python API.
  It contains no feature logic of its own.
- **Clients** (cfclient, cfcli, scripts) keep only UI and application-specific behaviour.

### Where does a feature go?

1. Does it encode or decode packets, memory contents, or interpret protocol replies? **crazyflie-lib.**
2. Is it built from crazyflie-lib calls, or independent of the Crazyflie (files, math), and would a
   second client need it? **crazyflie-ecosystem.**
3. Is it about how something looks or behaves in one specific application? **The client.**

## Contribute
Before you open a pull request, please read our [General guidelines for code contributions](https://www.bitcraze.io/development/contribute/general-guidelines/).

For other ways to contribute, see the [contribute page](https://www.bitcraze.io/development/contribute/).

[Crazyflie]: https://www.bitcraze.io/documentation/system/platform/
[crazyflie-lib]: https://github.com/bitcraze/crazyflie-lib-rs
[cflib2]: https://github.com/bitcraze/crazyflie-lib-python-v2