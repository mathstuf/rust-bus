use super::interface::DBusInterface;

extern crate core;
use self::core::ops::Deref;

extern crate dbus;
use self::dbus::{Connection, Error};

use std::collections::btree_map::BTreeMap;

pub struct DBusObject<'a> {
    ifaces: BTreeMap<String, Box<DBusInterface>>,

    conn: &'a Connection,
    path: String,
}

impl<'a> DBusObject<'a> {
    pub fn new(conn: &'a Connection, ifaces: BTreeMap<String, Box<DBusInterface>>, path: &str) -> Result<Self, Error> {
        try!(conn.register_object_path(path));

        Ok(DBusObject {
            ifaces: ifaces,

            conn: conn,
            path: path.to_string(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn get_interface(&self, interface: &str) -> Option<&DBusInterface> {
        self.ifaces.get(interface).map(|iface| {
            // FIXME: Why is .deref() necessary?
            iface.deref()
        })
    }

    pub fn get_interface_mut<'i>(&'i mut self, interface: &str) -> Option<&'i mut (DBusInterface + 'i)> {
        self.ifaces.get_mut(interface).map(|iface| ->
            // FIXME: Why is this not just iface.deref_mut().
            // FIXME: Why is .deref_mut() necessary?
            &'i mut DBusInterface { &mut **iface }
        )
    }
}

impl<'a> Drop for DBusObject<'a> {
    fn drop(&mut self) {
        self.conn.unregister_object_path(self.path())
    }
}
