extern crate dbus_bytestream;
use self::dbus_bytestream::connection;
use self::dbus_bytestream::demarshal;

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum DBusError {
    InvalidReply(String),
    ErrorMessage(connection::Error),
    NoServerName,

    ServerAlreadyRegistered(String),
    NoSuchServer(String),
    PathAlreadyRegistered(String),
    InvalidPath(String),
    NoSuchPath(String),
    ExtractArguments(demarshal::DemarshalError),
    InterfaceAlreadyRegistered(String),
}

impl Display for DBusError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            DBusError::InvalidReply(ref desc)               => write!(f, "invalid reply: {}", desc),
            DBusError::ErrorMessage(ref error)              => write!(f, "dbus error: {:?}", error),
            DBusError::NoServerName                         => write!(f, "listening server cannot handle methods"),
            DBusError::ServerAlreadyRegistered(ref server)  => write!(f, "server already registered: {}", server),
            DBusError::NoSuchServer(ref server)             => write!(f, "no such server: {}", server),
            DBusError::PathAlreadyRegistered(ref path)      => write!(f, "path already registered: {}", path),
            DBusError::InvalidPath(ref path)                => write!(f, "invalid path: {}", path),
            DBusError::NoSuchPath(ref path)                 => write!(f, "no such path: {}", path),
            DBusError::ExtractArguments(ref dmerr)          => write!(f, "failed to extract arguments: {}", dmerr),
            DBusError::InterfaceAlreadyRegistered(ref name) => write!(f, "interface already registered: {}", name),
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

impl From<connection::Error> for DBusError {
    fn from(error: connection::Error) -> Self {
        DBusError::ErrorMessage(error)
    }
}

impl From<demarshal::DemarshalError> for DBusError {
    fn from(error: demarshal::DemarshalError) -> Self {
        DBusError::ExtractArguments(error)
    }
}
