extern crate dbus;
use self::dbus::{Connection, NameFlag, ReleaseNameReply};

use super::error::DBusError;
use super::interface::DBusInterface;
use super::object::DBusObject;

use std::collections::btree_map::{BTreeMap, Entry};
use std::error::Error;

pub struct DBusServer<'a> {
    conn: &'a Connection,
    name: String,

    objects: BTreeMap<String, DBusObject<'a>>,
}

impl<'a> DBusServer<'a> {
    pub fn new(conn: &'a Connection, name: &str) -> Result<DBusServer<'a>, dbus::Error> {
        try!(conn.register_name(name, NameFlag::DoNotQueue as u32));

        Ok(DBusServer {
            conn: conn,
            name: name.to_string(),

            objects: BTreeMap::new(),
        })
    }

    pub fn add_object(&mut self, path: &str, ifaces: Vec<DBusInterface>) -> Result<&DBusObject<'a>, Box<Error>> {
        match self.objects.entry(path.to_string()) {
            Entry::Vacant(v)    => Ok(v.insert(try!(DBusObject::new(self.conn, ifaces, path)))),
            Entry::Occupied(_)  => Err(Box::new(DBusError::PathAlreadyRegistered(path.to_string()))),
        }
    }

    pub fn remove_object(&mut self, path: &str) -> Result<(), DBusError> {
        match self.objects.remove(path) {
            Some(_) => Ok(()),
            None    => Err(DBusError::NoSuchPath(path.to_string())),
        }
    }
}

impl<'a> Drop for DBusServer<'a> {
    fn drop(&mut self) {
        let res = self.conn.release_name(&self.name[..]);
        match res {
            Ok(reply) =>
                match reply {
                    ReleaseNameReply::Released    => (),
                    ReleaseNameReply::NonExistent => panic!("internal error: non-existent name {}?!", self.name),
                    ReleaseNameReply::NotOwner    => panic!("internal error: not the owner of {}?!", self.name),
                },
            Err(err) => println!("failed to release {}: {:?}: {:?}", self.name, err.name(), err.message()),
        }
    }
}
