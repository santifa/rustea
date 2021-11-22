/// rustea is a small cli tool to interact with git repositories hosted
/// by Gitea Instances. Copyright (C) 2021  Henrik Jürges (juerges.henrik@gmail.com)
///
/// This program is free software: you can redistribute it and/or modify
/// it under the terms of the GNU General Public License as published by
/// the Free Software Foundation, either version 3 of the License, or
/// (at your option) any later version.
///
/// This program is distributed in the hope that it will be useful,
/// but WITHOUT ANY WARRANTY; without even the implied warranty of
/// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
/// GNU General Public License for more details.
///
/// You should have received a copy of the GNU General Public License
/// along with this program. If not, see <https://www.gnu.org/licenses/>.
use core::fmt;
use std::{fmt::Display, io, num::ParseIntError};

use crate::gitea::gitea_api;

/// A `Result` alias where the `Err` case is `rustea::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// This is the main error type for rustea it encapsulates all the other types or errors.
#[derive(Debug)]
pub enum Error {
    Api(gitea_api::ApiError),
    Io(io::Error),
    Version(ParseIntError),
    Update(ureq::Error),
    Configuration(ConfigError),
    Rustea(String),
}

impl Error {
    pub fn io(kind: io::ErrorKind, msg: String) -> Self {
        Error::Io(io::Error::new(kind, msg))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    WriteError(toml::ser::Error),
    ReadError(toml::de::Error),
    LocationError,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::WriteError(e) => write!(f, "{}", e),
            ConfigError::ReadError(e) => write!(f, "{}", e),
            ConfigError::LocationError => write!(f, ""),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::Api(ref c) => Some(c),
            Error::Io(ref c) => Some(c),
            Error::Rustea(_) => None,
            Error::Configuration(_) => None,
            Error::Version(ref c) => Some(c),
            Error::Update(ref c) => Some(c),
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            Error::Api(ref c) => Some(c),
            Error::Io(ref c) => Some(c),
            Error::Rustea(_) => None,
            Error::Configuration(_) => None,
            Error::Version(ref c) => Some(c),
            Error::Update(ref c) => Some(c),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Api(e) => write!(f, "Gitea api error: {}", e),
            Error::Io(e) => write!(f, "IO Error: {}", e),
            Error::Rustea(e) => write!(f, "Error pushing configuration: {}", e),
            Error::Configuration(e) => match e {
                ConfigError::WriteError(_) => write!(f, "Failed to write configuration {}", e),
                ConfigError::ReadError(_) => write!(f, "Failed to read configuration {}", e),
                ConfigError::LocationError => write!(f, "Could not find home directory"),
            },
            Error::Version(e) => write!(f, "Failed to parse version: {}", e),
            Error::Update(e) => write!(f, "Update failed: {}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<gitea_api::ApiError> for Error {
    fn from(err: gitea_api::ApiError) -> Self {
        Error::Api(err)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Configuration(ConfigError::WriteError(err))
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Configuration(ConfigError::ReadError(err))
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::Version(err)
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Error::Update(err)
    }
}
