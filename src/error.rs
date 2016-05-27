extern crate dbus_bytestream;
use self::dbus_bytestream::connection;
use self::dbus_bytestream::demarshal;

use std::error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum Error {
    InvalidReply(String),
    ErrorMessage(connection::Error),
    NoServerName,

    ServerAlreadyRegistered(String),
    NoSuchServer(String),
    PathAlreadyRegistered(String),
    NoSuchPath(String),
    ExtractArguments(demarshal::DemarshalError),
    InterfaceAlreadyRegistered(String),
    InterfacesFinalized(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Error::InvalidReply(ref desc)               => write!(f, "invalid reply: {}", desc),
            Error::ErrorMessage(ref error)              => write!(f, "dbus error: {:?}", error),
            Error::NoServerName                         => write!(f, "listening server cannot handle methods"),
            Error::ServerAlreadyRegistered(ref server)  => write!(f, "server already registered: {}", server),
            Error::NoSuchServer(ref server)             => write!(f, "no such server: {}", server),
            Error::PathAlreadyRegistered(ref path)      => write!(f, "path already registered: {}", path),
            Error::NoSuchPath(ref path)                 => write!(f, "no such path: {}", path),
            Error::ExtractArguments(ref dmerr)          => write!(f, "failed to extract arguments: {}", dmerr),
            Error::InterfaceAlreadyRegistered(ref name) => write!(f, "interface already registered: {}", name),
            Error::InterfacesFinalized(ref name)        => write!(f, "interfaces have been finalized; cannot add {}", name),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "D-Bus error"
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::ErrorMessage(ref error) => Some(error),
            _ => None,
        }
    }
}

impl From<connection::Error> for Error {
    fn from(error: connection::Error) -> Self {
        Error::ErrorMessage(error)
    }
}

impl From<demarshal::DemarshalError> for Error {
    fn from(error: demarshal::DemarshalError) -> Self {
        Error::ExtractArguments(error)
    }
}
