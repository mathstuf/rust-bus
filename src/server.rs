extern crate dbus;
use self::dbus::{NameFlag, ReleaseNameReply};

use super::connection::DBusConnection;
use super::error::DBusError;
use super::interface::DBusInterfaceMap;
use super::message::{DBusMessage, DBusMessageType};
use super::object::DBusObject;
use super::target::DBusTarget;

use std::collections::btree_map::{BTreeMap, Entry};

pub type DBusSignalHandler = Box<FnMut(&DBusConnection, &DBusTarget) -> ()>;
type DBusSignalHandlers = Vec<DBusSignalHandler>;
type DBusSignalHandlerMap = BTreeMap<DBusTarget, DBusSignalHandlers>;

fn _add_handler(handlers: &mut DBusSignalHandlerMap, signal: DBusTarget, handler: DBusSignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v)    => { v.insert(vec![handler]); },
        Entry::Occupied(o)  => o.into_mut().push(handler),
    };
}

pub struct DBusServer<'a> {
    conn: &'a DBusConnection,
    name: String,

    objects: BTreeMap<String, DBusObject<'a>>,
    signals: DBusSignalHandlerMap,
    namespace_signals: DBusSignalHandlerMap,
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
        match self.objects.entry(path.to_owned()) {
            Entry::Vacant(v)    => {
                let obj = try!(DBusObject::new(self.conn, path, iface_map));

                v.insert(obj);

                Ok(())
            },
            Entry::Occupied(_)  => Err(DBusError::PathAlreadyRegistered(path.to_owned())),
        }.map(|_| self)
    }

    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self, DBusError> {
        match self.objects.remove(path) {
            Some(_) => Ok(self),
            None    => Err(DBusError::NoSuchPath(path.to_owned())),
        }
    }

    pub fn connect(&mut self, signal: DBusTarget, callback: DBusSignalHandler) -> Result<&mut Self, DBusError> {
        try!(self.conn.add_match(&format!("type='signal',interface='{}',path='{}',member='{}'",
                                          signal.interface,
                                          signal.object,
                                          signal.method)));

        _add_handler(&mut self.signals, signal, callback);

        Ok(self)
    }

    pub fn connect_namespace(&mut self, signal: DBusTarget, callback: DBusSignalHandler) -> Result<&mut Self, DBusError> {
        try!(self.conn.add_match(&format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                          signal.interface,
                                          signal.object,
                                          signal.method)));

        _add_handler(&mut self.namespace_signals, signal, callback);

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

    fn _match_signal<'b>(&mut self, m: &'b mut DBusMessage) -> &'b mut DBusMessage {
        let conn = (&self.conn).clone();

        DBusTarget::extract(m).map(|signal| {
            self.signals.get_mut(&signal).map(|handlers| {
                handlers.iter_mut().map(|f| {
                    f(conn, &signal);
                })
            });

            self.namespace_signals.iter_mut().filter(|&(expect, _)| {
                expect.namespace_eq(&signal)
            }).map(|(_, handlers)| {
                handlers.iter_mut().map(|f| {
                    f(conn, &signal);
                })
            }).collect::<Vec<_>>();
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
