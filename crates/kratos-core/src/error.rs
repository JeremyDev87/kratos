use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub type KratosResult<T> = Result<T, KratosError>;

#[derive(Debug)]
pub enum KratosError {
    Io(std::io::Error),
    Json(String),
    Config(String),
    InvalidReportVersion { expected: u32, found: u32 },
    NotImplemented { feature: &'static str },
}

impl KratosError {
    pub fn not_implemented(feature: &'static str) -> Self {
        Self::NotImplemented { feature }
    }
}

impl Display for KratosError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(message) => write!(f, "JSON error: {message}"),
            Self::Config(message) => write!(f, "Config error: {message}"),
            Self::InvalidReportVersion { expected, found } => {
                write!(f, "Invalid report version: expected {expected}, found {found}")
            }
            Self::NotImplemented { feature } => {
                write!(f, "Not implemented yet: {feature}")
            }
        }
    }
}

impl Error for KratosError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(_)
            | Self::Config(_)
            | Self::InvalidReportVersion { .. }
            | Self::NotImplemented { .. } => None,
        }
    }
}

impl From<std::io::Error> for KratosError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
