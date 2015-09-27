use super::connection::{DBusConnection, DBusReleaseNameReply, DBusRequestNameFlags};
use super::error::DBusError;
use super::interface::DBusInterfaceMap;
use super::message::DBusMessage;
use super::object::DBusObject;
use super::target::DBusTarget;

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

pub type DBusSignalHandler = Box<FnMut(&DBusConnection, &DBusTarget) -> ()>;
type DBusSignalHandlers = Vec<DBusSignalHandler>;
type DBusSignalHandlerMap = BTreeMap<DBusTarget, DBusSignalHandlers>;

fn _add_handler(handlers: &mut DBusSignalHandlerMap, signal: DBusTarget, handler: DBusSignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v)    => { v.insert(vec![handler]); },
        Entry::Occupied(o)  => o.into_mut().push(handler),
    };
}

pub struct DBusServer {
    conn: Rc<DBusConnection>,
    name: String,
    can_handle: bool,

    // TODO: store children information
    objects: DBusMap<DBusObject>,
    signals: DBusSignalHandlerMap,
    namespace_signals: DBusSignalHandlerMap,
}

impl DBusServer {
    pub fn new_listener(conn: Rc<DBusConnection>, name: &str) -> Result<DBusServer, DBusError> {
        Ok(DBusServer {
            conn: conn,
            name: name.to_owned(),
            can_handle: false,

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
            namespace_signals: BTreeMap::new(),
        })
    }

    pub fn new(conn: Rc<DBusConnection>, name: &str) -> Result<DBusServer, DBusError> {
        try!(conn.request_name(name, DBusRequestNameFlags::DoNotQueue));

        // TODO: add root object
        // TODO: add ObjectManager interface

        Ok(DBusServer {
            conn: conn,
            name: name.to_owned(),
            can_handle: true,

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
            namespace_signals: BTreeMap::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_object(&mut self, path: &str, iface_map: DBusInterfaceMap) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        match self.objects.entry(path.to_owned()) {
            Entry::Vacant(v)    => {
                // TODO: store this
                let children = Rc::new(RefCell::new(vec![]));
                let obj = try!(DBusObject::new(path, iface_map, children));

                // TODO: emit InterfacesAdded signal

                v.insert(obj);

                Ok(())
            },
            Entry::Occupied(_)  => Err(DBusError::PathAlreadyRegistered(path.to_owned())),
        }.map(|_| self)
    }

    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        match self.objects.remove(path) {
            Some(_) => {
                //TODO: emit InterfacesRemoved signal

                Ok(self)
            },
            None    => Err(DBusError::NoSuchPath(path.to_owned())),
        }
    }

    pub fn connect(&mut self, signal: DBusTarget, callback: DBusSignalHandler) -> Result<&mut Self, DBusError> {
        let dbus_match = format!("type='signal',interface='{}',path='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.signals, signal, callback);

        Ok(self)
    }

    pub fn connect_namespace(&mut self, signal: DBusTarget, callback: DBusSignalHandler) -> Result<&mut Self, DBusError> {
        let dbus_match = format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.namespace_signals, signal, callback);

        Ok(self)
    }

    pub fn handle_message<'b>(&mut self, m: &'b mut DBusMessage) -> Option<&'b mut DBusMessage> {
        if m.is_signal() {
            Some(self._match_signal(m))
        } else if m.is_method_call() {
            self._call_method(m)
        } else {
            Some(m)
        }
    }

    fn _call_method<'b>(&mut self, m: &'b mut DBusMessage) -> Option<&'b mut DBusMessage> {
        let conn = self.conn.clone();
        self.objects.iter_mut().fold(Some(m), |opt_m, (_, object)| {
            opt_m.and_then(|mut m| {
                match object.handle_message(&conn, &mut m) {
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
        let conn = self.conn.clone();

        DBusTarget::extract(m).map(|signal| {
            for handlers in self.signals.get_mut(&signal) {
                for handler in handlers.iter_mut() {
                    handler(&conn, &signal);
                }
            }

            let matched_handlers = self.namespace_signals.iter_mut().filter(|&(expect, _)| {
                expect.namespace_eq(&signal)
            });

            for (_, handlers) in matched_handlers {
                for handler in handlers.iter_mut() {
                    handler(&conn, &signal);
                };
            };
        });

        m
    }
}

impl Drop for DBusServer {
    fn drop(&mut self) {
        if !self.can_handle {
            return;
        }

        let res = self.conn.release_name(&self.name);
        match res {
            Ok(reply) =>
                match reply {
                    DBusReleaseNameReply::Released    => (),
                    DBusReleaseNameReply::NonExistent => panic!("internal error: non-existent name {}?!", self.name),
                    DBusReleaseNameReply::NotOwner    => panic!("internal error: not the owner of {}?!", self.name),
                },
            Err(err) => println!("failed to release {}: {:?}", self.name, err),
        }
    }
}
