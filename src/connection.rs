extern crate bitflags;

extern crate dbus_bytestream;
use self::dbus_bytestream::connection;

use super::error::Error;
use super::message::{Message, MessageType};
use super::value::{BasicValue, Value};

pub struct Connection {
    conn: connection::Connection,
}

bitflags! {
    pub flags RequestNameFlags: u32 {
        const ALLOW_REPLACEMENT = 0x1,
        const REPLACE_EXISTING  = 0x2,
        const DO_NOT_QUEUE      = 0x4,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RequestNameReply {
    PrimaryOwner,
    InQueue,
    Exists,
    AlreadyOwner,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReleaseNameReply {
    Released,
    NonExistent,
    NotOwner,
}

pub struct Messages<'a> {
    conn: &'a connection::Connection,
}

impl Connection {
    pub fn session_new() -> Result<Connection, Error> {
        Ok(Connection {
            conn: try!(connection::Connection::connect_session()),
        })
    }

    pub fn system_new() -> Result<Connection, Error> {
        Ok(Connection {
            conn: try!(connection::Connection::connect_system()),
        })
    }

    pub fn request_name(&self, name: &str, flags: RequestNameFlags) -> Result<RequestNameReply, Error> {
        let msg = Message::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "RequestName")
            .add_argument(&name)
            .add_argument(&flags.bits);
        if let Some(mut results) = try!(self.conn.call_sync(msg.extract())) {
            if let Some(Value::BasicValue(BasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(RequestNameReply::PrimaryOwner),
                    2 => Ok(RequestNameReply::InQueue),
                    3 => Ok(RequestNameReply::Exists),
                    4 => Ok(RequestNameReply::AlreadyOwner),
                    _ => Err(Error::InvalidReply(format!("RequestName: invalid response {}", r))),
                }
            } else {
                return Err(Error::InvalidReply("RequestName: invalid response".to_owned()));
            }
        } else {
            return Err(Error::InvalidReply("RequestName: no response".to_owned()));
        }
    }

    pub fn release_name(&self, name: &str) -> Result<ReleaseNameReply, Error> {
        let msg = Message::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "ReleaseName")
            .add_argument(&name);
        if let Some(mut results) = try!(self.conn.call_sync(msg.extract())) {
            if let Some(Value::BasicValue(BasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(ReleaseNameReply::Released),
                    2 => Ok(ReleaseNameReply::NonExistent),
                    3 => Ok(ReleaseNameReply::NotOwner),
                    _ => Err(Error::InvalidReply(format!("ReleaseName: invalid response {}", r))),
                }
            } else {
                return Err(Error::InvalidReply("ReleaseName: invalid response".to_owned()));
            }
        } else {
            return Err(Error::InvalidReply("ReleaseName: no response".to_owned()));
        }
    }

    pub fn add_match(&self, match_rule: &str) -> Result<(), Error> {
        let msg = Message::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "AddMatch")
            .add_argument(&match_rule);
        try!(self.conn.call_sync(msg.extract()));
        Ok(())
    }

    pub fn send(&self, msg: Message) -> Result<u32, Error> {
        Ok(try!(self.conn.send(msg.extract())))
    }

    pub fn iter(&self) -> Messages {
        Messages {
            conn: &self.conn,
        }
    }
}

fn _should_handle(message: &Message) -> bool {
    match message.message_type() {
        MessageType::MethodCall => true,
        MessageType::Signal     => true,
        _                       => false,
    }
}

impl<'a> Iterator for Messages<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.conn.read_msg();
        match res {
            Ok(message) => {
                let dbus_message = Message::new(message);
                if _should_handle(&dbus_message) {
                    Some(dbus_message)
                } else {
                    None
                }
            },
            Err(_)      => None,
        }
    }
}
