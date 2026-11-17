/// [Result] alias for return types of the crate API
pub type Result<T> = std::result::Result<T, Error>;

/// Error enum type
#[derive(Debug)]
pub enum Error {
    /// Error returned by [crazyflie-lib](crazyflie_lib).
    LibError(crazyflie_lib::Error),
    /// A configuration file could not be parsed or failed validation. The String contains the reason.
    ConfigFileError(String),
    /// Lighthouse operation failed. The String contains the reason.
    LighthouseError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::LibError(e) => write!(f, "Crazyflie lib error: {}", e),
            Error::ConfigFileError(msg) => write!(f, "Config file error: {}", msg),
            Error::LighthouseError(msg) => write!(f, "Lighthouse error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::LibError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<crazyflie_lib::Error> for Error {
    fn from(e: crazyflie_lib::Error) -> Self {
        Error::LibError(e)
    }
}
