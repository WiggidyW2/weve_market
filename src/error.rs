use serde_json;
use tonic;

#[derive(Debug)]
pub enum Error {
    EnvSocketParseError(std::net::AddrParseError),
    EnvIntParseError(std::num::ParseIntError),
    EnvJsonParseError(serde_json::Error),
    EnvReadError(std::env::VarError),
    ServiceServeError(tonic::transport::Error),
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Error::EnvReadError(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::EnvIntParseError(err)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(err: std::net::AddrParseError) -> Self {
        Error::EnvSocketParseError(err)
    }
}
