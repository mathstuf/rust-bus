use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum DBusError {
    PathAlreadyRegistered(String),
    NoSuchPath(String),
}

impl Display for DBusError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            DBusError::PathAlreadyRegistered(ref path) => write!(f, "path already registered: {}", path),
            DBusError::NoSuchPath(ref path)            => write!(f, "no such path: {}", path),
        }
    }
}

impl Error for DBusError {
    fn description(&self) -> &str {
        "D-Bus error"
    }
}
