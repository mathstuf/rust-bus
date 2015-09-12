extern crate dbus;
use self::dbus::{Connection, ConnectionItem, Message, NameFlag, ReleaseNameReply};
use self::dbus::obj::ObjectPath;

use super::error::DBusError;
use super::target::DBusTarget;

use std::collections::btree_map::{BTreeMap, Entry};
use std::error::Error;

pub struct DBusServer<'a> {
    conn: &'a Connection,
    name: String,

    objects: BTreeMap<String, ObjectPath<'a>>,
    signals: BTreeMap<DBusTarget, Vec<fn (&Connection, &DBusTarget) -> ()>>
}

impl<'a> DBusServer<'a> {
    pub fn new(conn: &'a Connection, name: &str) -> Result<DBusServer<'a>, dbus::Error> {
        try!(conn.register_name(name, NameFlag::DoNotQueue as u32));

        Ok(DBusServer {
            conn: conn,
            name: name.to_owned(),

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
        })
    }

    pub fn add_object(&mut self, path: &str, add_interfaces: fn (&mut ObjectPath<'a>) -> ()) -> Result<(), Box<Error>> {
        match self.objects.entry(path.to_owned()) {
            Entry::Vacant(v)    => {
                let mut obj = ObjectPath::new(self.conn, path, true);
                try!(obj.set_registered(true));

                // Add interfaces.
                add_interfaces(&mut obj);

                v.insert(obj);

                Ok(())
            },
            Entry::Occupied(_)  => Err(Box::new(DBusError::PathAlreadyRegistered(path.to_owned()))),
        }
    }

    pub fn remove_object(&mut self, path: &str) -> Result<(), DBusError> {
        match self.objects.remove(path) {
            Some(_) => Ok(()),
            None    => Err(DBusError::NoSuchPath(path.to_owned())),
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
        self.objects.iter_mut().fold(Some(m), |opt_m, (_, object)| {
            opt_m.and_then(|mut m| {
                match object.handle_message(&mut m) {
                    None          => Some(m),
                    Some(Ok(()))  => None,
                    Some(Err(())) => {
                        println!("failed to send a reply for {:?}", m);
                        None
                    },
                }
            })
        });
    }

    fn _match_signal(&self, m: Message) -> () {
        let ref signals = self.signals;
        let conn = self.conn;

        DBusTarget::extract(&m).and_then(|signal| {
            signals.get(&signal).map(|fs| {
                fs.iter().fold((conn, signal), |(conn, signal), f| {
                    f(&conn, &signal);
                    (conn, signal)
                })
            })
        });
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
