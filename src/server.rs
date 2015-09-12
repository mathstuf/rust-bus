extern crate dbus;
use self::dbus::{NameFlag, ReleaseNameReply};

use super::connection::DBusConnection;
use super::error::DBusError;
use super::interface::DBusInterfaceMap;
use super::message::{DBusMessage, DBusMessageType};
use super::object::DBusObject;
use super::target::DBusTarget;

use std::collections::btree_map::{BTreeMap, Entry};

pub struct DBusServer<'a> {
    conn: &'a DBusConnection,
    name: String,

    objects: BTreeMap<String, DBusObject<'a>>,
    signals: BTreeMap<DBusTarget, Vec<fn (&DBusConnection, &DBusTarget) -> ()>>,
    namespace_signals: BTreeMap<DBusTarget, Vec<fn (&DBusConnection, &DBusTarget) -> ()>>,
}

impl<'a> DBusServer<'a> {
    pub fn new(conn: &'a DBusConnection, name: &str) -> Result<DBusServer<'a>, DBusError> {
        try!(conn.register_name(name, NameFlag::DoNotQueue as u32));

        Ok(DBusServer {
            conn: conn,
            name: name.to_owned(),

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
            namespace_signals: BTreeMap::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_object(&mut self, path: &str, iface_map: DBusInterfaceMap<'a>) -> Result<&mut Self, DBusError> {
        // XXX: Use `.map` when type resolution works better. Currently, `.map` causes the caller
        // type to be set in stone due to eager type resolution and no fluidity in setting it. This
        // causes the type to be too concrete on the caller side which then fails to map to the end
        // result type of `.map` even though `.map` is "transparent" to the error type.
        Result::map(match self.objects.entry(path.to_owned()) {
            Entry::Vacant(v)    => {
                let obj = try!(DBusObject::new(self.conn, path, iface_map));

                v.insert(obj);

                Ok(())
            },
            Entry::Occupied(_)  => Err(DBusError::PathAlreadyRegistered(path.to_owned())),
        }, |_| self)
    }

    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self, DBusError> {
        match self.objects.remove(path) {
            Some(_) => Ok(self),
            None    => Err(DBusError::NoSuchPath(path.to_owned())),
        }
    }

    pub fn connect(&mut self, signal: DBusTarget, callback: fn (&DBusConnection, &DBusTarget) -> ()) -> Result<&mut Self, DBusError> {
        try!(self.conn.add_match(&format!("type='signal',interface='{}',path='{}',member='{}'",
                                          signal.interface,
                                          signal.object,
                                          signal.method)));

        match self.signals.entry(signal) {
            Entry::Vacant(v)    => { v.insert(vec![callback]); },
            Entry::Occupied(o)  => o.into_mut().push(callback),
        };

        Ok(self)
    }

    pub fn connect_namespace(&mut self, signal: DBusTarget, callback: fn (&DBusConnection, &DBusTarget) -> ()) -> Result<&mut Self, DBusError> {
        try!(self.conn.add_match(&format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                          signal.interface,
                                          signal.object,
                                          signal.method)));

        match self.namespace_signals.entry(signal) {
            Entry::Vacant(v)    => { v.insert(vec![callback]); },
            Entry::Occupied(o)  => o.into_mut().push(callback),
        };

        Ok(self)
    }

    pub fn handle_message<'b>(&mut self, m: &'b mut DBusMessage) -> Option<&'b mut DBusMessage> {
        match m.msg_type() {
            DBusMessageType::Signal     => Some(self._match_signal(m)),
            DBusMessageType::MethodCall => self._call_method(m),
            _                           => Some(m),
        }
    }

    fn _call_method<'b>(&mut self, m: &'b mut DBusMessage) -> Option<&'b mut DBusMessage> {
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
        })
    }

    fn _match_signal<'b>(&self, m: &'b mut DBusMessage) -> &'b mut DBusMessage {
        DBusTarget::extract(m).map(|signal| {
            self.signals.get(&signal).map(|fs| {
                fs.iter().map(|f| {
                    f(&self.conn, &signal);
                })
            });

            self.namespace_signals.get(&signal).map(|fs| {
                fs.iter().map(|f| {
                    f(&self.conn, &signal);
                })
            });
        });

        m
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
