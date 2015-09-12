extern crate dbus;
use self::dbus::{BusType, Connection};

use super::error::DBusError;

pub struct DBusConnection {
    conn: Connection,
}

impl DBusConnection {
    pub fn session_new() -> Result<DBusConnection, DBusError> {
        Ok(DBusConnection {
            conn: try!(Connection::get_private(BusType::Session)),
        })
    }

    pub fn system_new() -> Result<DBusConnection, DBusError> {
        Ok(DBusConnection {
            conn: try!(Connection::get_private(BusType::System)),
        })
    }

    pub fn _connection(&self) -> &Connection {
        &self.conn
    }
}
