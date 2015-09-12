extern crate dbus;

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum DBusError {
    ErrorMessage(dbus::Error),

    ServerAlreadyRegistered(String),
    NoSuchServer(String),
    PathAlreadyRegistered(String),
    NoSuchPath(String),
}

impl Display for DBusError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            DBusError::ErrorMessage(ref error)              => write!(f, "dbus error: {:?}: {:?}", error.name(), error.message()),
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

    fn cause(&self) -> Option<&Error> {
        match *self {
            DBusError::ErrorMessage(ref error) => Some(error),
            _ => None,
        }
    }
}

impl From<dbus::Error> for DBusError {
    fn from(error: dbus::Error) -> Self {
        DBusError::ErrorMessage(error)
    }
}
