extern crate dbus_bytestream;
use self::dbus_bytestream::connection;
use self::dbus_bytestream::demarshal;

use std::error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
/// Error states.
pub enum Error {
    /// An invalid reply was received from a method call.
    InvalidReply(String),
    /// An error message from the underlying D-Bus communication.
    ErrorMessage(connection::Error),
    /// An object was added to a signal-receiver server.
    NoServerName,

    /// A server with the given name was already registered.
    ServerAlreadyRegistered(String),
    /// A request for a non-existent server was given.
    NoSuchServer(String),
    /// An object was registered to a path, but it already existed.
    PathAlreadyRegistered(String),
    /// An object was given an invalid path.
    InvalidPath(String),
    /// An object was requested to be removed, but it does not exist.
    NoSuchPath(String),
    /// Extracting values from a message body failed.
    ExtractArguments(demarshal::DemarshalError),
    /// An attempt to redefine an interface for an object was made.
    InterfaceAlreadyRegistered(String),
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
            Error::InvalidPath(ref path)                => write!(f, "invalid path: {}", path),
            Error::NoSuchPath(ref path)                 => write!(f, "no such path: {}", path),
            Error::ExtractArguments(ref dmerr)          => write!(f, "failed to extract arguments: {}", dmerr),
            Error::InterfaceAlreadyRegistered(ref name) => write!(f, "interface already registered: {}", name),
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
            _                              => None,
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
