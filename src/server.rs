extern crate dbus;
use self::dbus::{Connection, ConnectionItem, Message, NameFlag, ReleaseNameReply};

use super::error::DBusError;
use super::interface::DBusInterface;
use super::object::DBusObject;
use super::target::{DBusTarget, extract_target};

use std::collections::btree_map::{BTreeMap, Entry};
use std::error::Error;

pub struct DBusServer<'a> {
    conn: &'a Connection,
    name: String,

    objects: BTreeMap<String, DBusObject<'a>>,
    signals: BTreeMap<DBusTarget, Vec<fn (&Connection, &DBusTarget) -> ()>>
}

impl<'a> DBusServer<'a> {
    pub fn new(conn: &'a Connection, name: &str) -> Result<DBusServer<'a>, dbus::Error> {
        try!(conn.register_name(name, NameFlag::DoNotQueue as u32));

        Ok(DBusServer {
            conn: conn,
            name: name.to_string(),

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
        })
    }

    pub fn add_object(&mut self, path: &str, ifaces: BTreeMap<String, Box<DBusInterface>>) -> Result<&DBusObject<'a>, Box<Error>> {
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

    pub fn connect(&mut self, signal: DBusTarget, callback: fn (&Connection, &DBusTarget) -> ()) -> () {
        match self.signals.entry(signal) {
            Entry::Vacant(v)    => { v.insert(vec![callback]); },
            Entry::Occupied(o)  => o.into_mut().push(callback),
        };
    }

    pub fn run(&mut self) -> () {
        self.conn.iter(100).fold((), |_, item| {
            match item {
                ConnectionItem::MethodCall(m)   => self._call_method(m),
                ConnectionItem::Signal(s)       => self._match_signal(s),
                ConnectionItem::Nothing         => (),
            }
        });
    }

    fn _call_method(&mut self, m: Message) -> () {
        let conn = self.conn;

        self._find_interface(&m).map(|(ref mut iface, ref method)| {
            iface.call_method(&method, conn, &m).map(|result| {
                conn.send(result)
            })
        });
    }

    fn _find_interface(&mut self, m: &Message) -> Option<(&mut DBusInterface, String)> {
        let ref mut objects = self.objects;

        extract_target(&m).and_then(move |method| {
            objects.get_mut(&method.1).and_then(|dbus_object| {
                dbus_object.get_interface_mut(&method.0).map(|dbus_interface| {
                    (dbus_interface, method.2.clone())
                })
            })
        })
    }

    fn _match_signal(&self, m: Message) -> () {
        // TODO: Implement.
    }
}

impl<'a> Drop for DBusServer<'a> {
    fn drop(&mut self) {
        let res = self.conn.release_name(&self.name);
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
