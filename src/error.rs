use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum DBusError {
    ServerAlreadyRegistered(String),
    NoSuchServer(String),
    PathAlreadyRegistered(String),
    NoSuchPath(String),
}

impl Display for DBusError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            DBusError::ServerAlreadyRegistered(ref server)  => write!(f, "server already registered: {}", server),
            DBusError::NoSuchServer(ref server)             => write!(f, "no such server: {}", server),
            DBusError::PathAlreadyRegistered(ref path)      => write!(f, "path already registered: {}", path),
            DBusError::NoSuchPath(ref path)                 => write!(f, "no such path: {}", path),
        }
    }
}

impl Error for DBusError {
    fn description(&self) -> &str {
        "D-Bus error"
    }
}
