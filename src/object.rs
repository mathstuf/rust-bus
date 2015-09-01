use super::interface::DBusInterface;

extern crate dbus;
use self::dbus::{Connection, Error};

pub struct DBusObject<'a, T: DBusInterface> {
    conn: &'a Connection,
    iface: T,
    path: String,
}

impl<'a, T: DBusInterface> DBusObject<'a, T> {
    pub fn new(conn: &'a Connection, iface: T, path: &str) -> Result<Self, dbus::Error> {
        try!(conn.register_object_path(path));

        Ok(DBusObject {
            conn: conn,
            iface: iface,
            path: path.to_string(),
        })
    }

    pub fn interface(&self) -> &T {
        &self.iface
    }

    pub fn interface_mut(&mut self) -> &mut T {
        &mut self.iface
    }

    pub fn path(&self) -> &str {
        &self.path[..]
    }
}

impl<'a, T: DBusInterface> Drop for DBusObject<'a, T> {
    fn drop(&mut self) {
        self.conn.unregister_object_path(self.path())
    }
}
