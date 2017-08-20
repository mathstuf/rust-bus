// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use crates::core::ops::DerefMut;

use connection::{Connection, ReleaseNameReply, DO_NOT_QUEUE};
use error::*;
use interface::InterfacesBuilder;
use message::{Message, MessageType};
use object::Object;
use target::Target;

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

type SignalHandler = Rc<RefCell<FnMut(&Connection, &Target) -> ()>>;
type SignalHandlers = Vec<SignalHandler>;
type SignalHandlerMap = BTreeMap<Target, SignalHandlers>;

fn _add_handler(handlers: &mut SignalHandlerMap, signal: Target, handler: SignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v) => {
            v.insert(vec![handler]);
        },
        Entry::Occupied(o) => o.into_mut().push(handler),
    };
}

/// A representation of a collection of objects which implement an interface.
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
    /// Create a new `Server` to listen for signals.
    pub fn new_listener(conn: Rc<Connection>, name: &str) -> Result<Self> {
        Ok(Server {
            conn: conn,
            name: name.to_owned(),
            can_handle: false,

            objects: BTreeMap::new(),
            signals: SignalHandlerMap::new(),
            namespace_signals: SignalHandlerMap::new(),
        })
    }

    /// Create a new `Server` to handle method calls from the bus.
    pub fn new(conn: Rc<Connection>, name: &str) -> Result<Self> {
        try!(conn.request_name(name, DO_NOT_QUEUE));

        // TODO: Add match for the server.
        // TODO: add root object
        // TODO: add ObjectManager interface

        Ok(Server {
            conn: conn,
            name: name.to_owned(),
            can_handle: true,

            objects: BTreeMap::new(),
            signals: SignalHandlerMap::new(),
            namespace_signals: SignalHandlerMap::new(),
        })
    }

    /// The name of the server.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add an object to the server with the given interfaces.
    pub fn add_object(&mut self, path: &str, ifaces: InterfacesBuilder) -> Result<&mut Self> {
        if !self.can_handle {
            bail!(ErrorKind::NoServerName);
        }

        // TODO: Validate the path is valid.

        match self.objects.entry(path.to_owned()) {
                Entry::Vacant(v) => {
                    // TODO: store this
                    let children = Rc::new(RefCell::new(vec![]));
                    let finalized_ifaces = try!(ifaces.finalize(&children));
                    let obj = Object::new(path, finalized_ifaces);

                    // TODO: emit InterfacesAdded signal

                    v.insert(obj);

                    Ok(())
                },
                Entry::Occupied(_) => bail!(ErrorKind::PathAlreadyRegistered(path.to_owned())),
            }
            .map(|_| self)
    }

    /// Remove an object from the server.
    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self> {
        if !self.can_handle {
            bail!(ErrorKind::NoServerName);
        }

        match self.objects.remove(path) {
            Some(_) => {
                // TODO: emit InterfacesRemoved signal

                Ok(self)
            },
            None => bail!(ErrorKind::NoSuchPath(path.to_owned())),
        }
    }

    /// Connect a handler to a specific object's signal.
    ///
    /// This will register a callback to listen to a specific object's signals.
    pub fn connect<F>(&mut self, signal: Target, callback: F) -> Result<&mut Self>
        where F: FnMut(&Connection, &Target) -> () + 'static
    {
        let dbus_match = format!("type='signal',interface='{}',path='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.signals, signal, Rc::new(RefCell::new(callback)));

        Ok(self)
    }

    /// Connect a handler to a set of objects' signals.
    ///
    /// Any object underneath the requested object path's hierarchy emitting the requested signal
    /// will trigger the callback.
    pub fn connect_namespace<F>(&mut self, signal: Target, callback: F) -> Result<&mut Self>
        where F: FnMut(&Connection, &Target) -> () + 'static
    {
        let dbus_match = format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.namespace_signals,
                     signal,
                     Rc::new(RefCell::new(callback)));

        Ok(self)
    }

    /// Handle a message with the appropriate handler.
    ///
    /// Returns `None` if the message was consumed, otherwise it returns the original message for
    /// further processing.
    pub fn handle_message<'b>(&self, m: &'b mut Message) -> Option<&'b mut Message> {
        match m.message_type() {
            MessageType::MethodCall => self._call_method(m),
            MessageType::Signal => Some(self._match_signal(m)),
            _ => Some(m),
        }
    }

    fn _call_method<'b>(&self, m: &'b mut Message) -> Option<&'b mut Message> {
        let conn = self.conn.clone();
        self.objects.iter().fold(Some(m), |opt_m, (_, object)| {
            opt_m.and_then(|mut m| {
                match object.handle_message(&conn, &mut m) {
                    None => Some(m),
                    Some(Ok(())) => None,
                    Some(Err(())) => {
                        println!("failed to send a reply for {:?}", m);
                        None
                    },
                }
            })
        })
    }

    fn _match_signal<'b>(&self, m: &'b mut Message) -> &'b mut Message {
        let conn = self.conn.clone();

        Target::extract(m).map(|signal| {
            for handlers in self.signals.get(&signal) {
                for handler in handlers.iter() {
                    let mut cb = handler.borrow_mut();

                    cb.deref_mut()(&conn, &signal);
                }
            }

            let matched_handlers =
                self.namespace_signals.iter().filter(|&(expect, _)| expect.namespace_eq(&signal));

            for (_, handlers) in matched_handlers {
                for handler in handlers.iter() {
                    let mut cb = handler.borrow_mut();

                    cb.deref_mut()(&conn, &signal);
                }
            }
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
            Ok(reply) => {
                match reply {
                    ReleaseNameReply::Released => (),
                    ReleaseNameReply::NonExistent => {
                        panic!("internal error: non-existent name {}?!", self.name)
                    },
                    ReleaseNameReply::NotOwner => {
                        panic!("internal error: not the owner of {}?!", self.name)
                    },
                }
            },
            Err(err) => println!("failed to release {}: {:?}", self.name, err),
        }
    }
}
