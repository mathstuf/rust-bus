extern crate bitflags;

extern crate dbus_bytestream;
use self::dbus_bytestream::connection::Connection;

use super::error::DBusError;
use super::message::DBusMessage;
use super::value::{DBusBasicValue, DBusValue};

pub struct DBusConnection {
    conn: Connection,
}

pub enum DBusRequestNameFlags {
    AllowReplacement = 0x1,
    ReplaceExisting  = 0x2,
    DoNotQueue       = 0x4,
}

pub enum DBusRequestNameReply {
    PrimaryOwner,
    InQueue,
    Exists,
    AlreadyOwner,
}

pub enum DBusReleaseNameReply {
    Released,
    NonExistent,
    NotOwner,
}

pub struct DBusMessages<'a> {
    conn: &'a Connection,
}

impl DBusConnection {
    pub fn session_new() -> Result<DBusConnection, DBusError> {
        Ok(DBusConnection {
            conn: try!(Connection::connect_session()),
        })
    }

    pub fn system_new() -> Result<DBusConnection, DBusError> {
        Ok(DBusConnection {
            conn: try!(Connection::connect_system()),
        })
    }

    pub fn request_name(&self, name: &str, flags: DBusRequestNameFlags) -> Result<DBusRequestNameReply, DBusError> {
        let msg = DBusMessage::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "RequestName")
            .add_argument(&name)
            .add_argument(&(flags as u32));
        if let Some(mut results) = try!(self.conn.call_sync(msg.extract())) {
            if let Some(DBusValue::BasicValue(DBusBasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(DBusRequestNameReply::PrimaryOwner),
                    2 => Ok(DBusRequestNameReply::InQueue),
                    3 => Ok(DBusRequestNameReply::Exists),
                    4 => Ok(DBusRequestNameReply::AlreadyOwner),
                    _ => Err(DBusError::InvalidReply(format!("RequestName: invalid response {}", r))),
                }
            } else {
                return Err(DBusError::InvalidReply("RequestName: invalid response".to_owned()));
            }
        } else {
            return Err(DBusError::InvalidReply("RequestName: no response".to_owned()));
        }
    }

    pub fn release_name(&self, name: &str) -> Result<DBusReleaseNameReply, DBusError> {
        let msg = DBusMessage::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "ReleaseName")
            .add_argument(&name);
        if let Some(mut results) = try!(self.conn.call_sync(msg.extract())) {
            if let Some(DBusValue::BasicValue(DBusBasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(DBusReleaseNameReply::Released),
                    2 => Ok(DBusReleaseNameReply::NonExistent),
                    3 => Ok(DBusReleaseNameReply::NotOwner),
                    _ => Err(DBusError::InvalidReply(format!("ReleaseName: invalid response {}", r))),
                }
            } else {
                return Err(DBusError::InvalidReply("ReleaseName: invalid response".to_owned()));
            }
        } else {
            return Err(DBusError::InvalidReply("ReleaseName: no response".to_owned()));
        }
    }

    pub fn add_match(&self, match_rule: &str) -> Result<(), DBusError> {
        let msg = DBusMessage::new_method_call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "AddMatch")
            .add_argument(&match_rule);
        try!(self.conn.call_sync(msg.extract()));
        Ok(())
    }

    pub fn iter(&self) -> DBusMessages {
        DBusMessages {
            conn: &self.conn,
        }
    }
}

impl<'a> Iterator for DBusMessages<'a> {
    type Item = DBusMessage;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.conn.read_msg();
        match res {
            Ok(message) => {
                let dbus_message = DBusMessage::new(message);
                if dbus_message.should_handle() {
                    Some(dbus_message)
                } else {
                    None
                }
            },
            Err(_)      => None,
        }
    }
}
