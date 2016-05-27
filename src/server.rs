use super::connection::{Connection, ReleaseNameReply, RequestNameFlags};
use super::error::Error;
use super::interface::Interfaces;
use super::message::Message;
use super::object::Object;
use super::target::Target;

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

pub type SignalHandler = Box<FnMut(&Connection, &Target) -> ()>;
type SignalHandlers = Vec<SignalHandler>;
type SignalHandlerMap = BTreeMap<Target, SignalHandlers>;

fn _add_handler(handlers: &mut SignalHandlerMap, signal: Target, handler: SignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v)    => { v.insert(vec![handler]); },
        Entry::Occupied(o)  => o.into_mut().push(handler),
    };
}

pub struct Server {
    conn: Rc<Connection>,
    name: String,
    can_handle: bool,

    // TODO: store children information
    objects: BTreeMap<String, Object>,
    signals: SignalHandlerMap,
    namespace_signals: SignalHandlerMap,
}

impl Server {
    pub fn new_listener(conn: Rc<Connection>, name: &str) -> Result<Server, Error> {
        Ok(Server {
            conn: conn,
            name: name.to_owned(),
            can_handle: false,

            objects: BTreeMap::new(),
            signals: BTreeMap::new(),
            namespace_signals: BTreeMap::new(),
        })
    }

    pub fn new(conn: Rc<Connection>, name: &str) -> Result<Server, Error> {
        try!(conn.request_name(name, RequestNameFlags::DoNotQueue));

        // TODO: add root object
        // TODO: add ObjectManager interface

        Ok(Server {
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

    pub fn add_object(&mut self, path: &str, iface_map: Interfaces) -> Result<&mut Self, Error> {
        if !self.can_handle {
            return Err(Error::NoServerName);
        }

        match self.objects.entry(path.to_owned()) {
            Entry::Vacant(v)    => {
                // TODO: store this
                let children = Rc::new(RefCell::new(vec![]));
                let obj = try!(Object::new(path, iface_map, children));

                // TODO: emit InterfacesAdded signal

                v.insert(obj);

                Ok(())
            },
            Entry::Occupied(_)  => Err(Error::PathAlreadyRegistered(path.to_owned())),
        }.map(|_| self)
    }

    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self, Error> {
        if !self.can_handle {
            return Err(Error::NoServerName);
        }

        match self.objects.remove(path) {
            Some(_) => {
                // TODO: emit InterfacesRemoved signal

                Ok(self)
            },
            None    => Err(Error::NoSuchPath(path.to_owned())),
        }
    }

    pub fn connect(&mut self, signal: Target, callback: SignalHandler) -> Result<&mut Self, Error> {
        let dbus_match = format!("type='signal',interface='{}',path='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.signals, signal, callback);

        Ok(self)
    }

    pub fn connect_namespace(&mut self, signal: Target, callback: SignalHandler) -> Result<&mut Self, Error> {
        let dbus_match = format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.namespace_signals, signal, callback);

        Ok(self)
    }

    pub fn handle_message<'b>(&mut self, m: &'b mut Message) -> Option<&'b mut Message> {
        if m.is_signal() {
            Some(self._match_signal(m))
        } else if m.is_method_call() {
            self._call_method(m)
        } else {
            Some(m)
        }
    }

    fn _call_method<'b>(&mut self, m: &'b mut Message) -> Option<&'b mut Message> {
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

    fn _match_signal<'b>(&mut self, m: &'b mut Message) -> &'b mut Message {
        let conn = self.conn.clone();

        Target::extract(m).map(|signal| {
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

impl Drop for Server {
    fn drop(&mut self) {
        if !self.can_handle {
            return;
        }

        let res = self.conn.release_name(&self.name);
        match res {
            Ok(reply) =>
                match reply {
                    ReleaseNameReply::Released    => (),
                    ReleaseNameReply::NonExistent => panic!("internal error: non-existent name {}?!", self.name),
                    ReleaseNameReply::NotOwner    => panic!("internal error: not the owner of {}?!", self.name),
                },
            Err(err) => println!("failed to release {}: {:?}", self.name, err),
        }
    }
}
