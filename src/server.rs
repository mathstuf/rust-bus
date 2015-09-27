extern crate core;
use self::core::ops::DerefMut;

use super::connection::{Connection, ReleaseNameReply, DO_NOT_QUEUE};
use super::error::Error;
use super::interface::{Map, ChildrenList, Interface, InterfacesBuilder};
use super::message::{Message, MessageType};
use super::object::Object;
use super::target::Target;

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

type SignalHandler = Rc<RefCell<FnMut(&Connection, &Target) -> ()>>;
type SignalHandlers = Vec<SignalHandler>;
type SignalHandlerMap = BTreeMap<Target, SignalHandlers>;

fn _add_handler(handlers: &mut SignalHandlerMap, signal: Target, handler: SignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v)    => { v.insert(vec![handler]); },
        Entry::Occupied(o)  => o.into_mut().push(handler),
    };
}

struct ObjectTreeCursor<'a> {
    tree: &'a mut ObjectTree,
}

impl<'a> ObjectTreeCursor<'a> {
    pub fn new(tree: &'a mut ObjectTree) -> Self {
        ObjectTreeCursor {
            tree: tree,
        }
    }

    pub fn tree(&self) -> &ObjectTree {
        self.tree
    }

    pub fn has_object(&self) -> bool {
        self.tree.object.is_some()
    }

    pub fn set_object(&mut self, object: Object, manager: bool) {
        self.tree.object = Some(object);
        self.tree.manager = manager;
    }

    pub fn find_or_create(self, name: &str) -> Self {
        ObjectTreeCursor::new(self.tree.find_or_create_object(name))
    }

    pub fn find(self, name: &str) -> Option<Self> {
        self.tree.find_object(name).map(ObjectTreeCursor::new)
    }

    pub fn remove(self, name: &str) -> Option<Object> {
        self.tree.remove_object(name)
    }
}

struct ObjectManagerInterface;

impl ObjectManagerInterface {
    pub fn new() -> Interface {
        unimplemented!()
    }
}

struct ObjectTree {
    object: Option<Object>,
    manager: bool,
    children_names: ChildrenList,
    children: Map<ObjectTree>,
}

impl ObjectTree {
    pub fn new() -> Self {
        ObjectTree {
            object: None,
            manager: false,
            children_names: Rc::new(RefCell::new(vec![])),
            children: Map::new(),
        }
    }

    pub fn find_object(&mut self, name: &str) -> Option<&mut Self> {
        self.children.get_mut(name)
    }

    pub fn find_or_create_object(&mut self, name: &str) -> &mut Self {
        let children_mod = self.children_names.clone();

        self.children.entry(name.to_owned())
            .or_insert_with(move || {
                children_mod.borrow_mut().push(name.to_owned());
                ObjectTree::new()
            })
    }

    pub fn remove_object(&mut self, name: &str) -> Option<Object> {
        match self.children.entry(name.to_owned()) {
            Entry::Vacant(_)    => None,
            Entry::Occupied(o)  => {
                if o.object.is_none() {
                    return None;
                }

                let object = o.object;

                if o.children.empty() {
                } else {
                    o.object = None;
                }

                o
            },
        }
    }

    pub fn insert(&mut self, path: String, ifaces: InterfacesBuilder, manager: bool) -> Result<(), Error> {
        if !path.starts_with("/") {
            return Err(Error::InvalidPath(path));
        }

        let top_cursor = ObjectTreeCursor::new(self);

        let mut ins_cursor = try!(path.split("/").skip(1).fold(Ok(top_cursor), |res_cursor, component| {
            res_cursor.and_then(|cursor| {
                if component.is_empty() {
                    return Err(Error::InvalidPath(path.clone()));
                }

                Ok(cursor.find_or_create(component))
            })
        }));

        if ins_cursor.has_object() {
            return Err(Error::PathAlreadyRegistered(path));
        }

        let final_ifaces = if manager {
            try!(ifaces.add_interface("org.freedesktop.DBus.ObjectManager", ObjectManagerInterface::new()))
        } else {
            ifaces
        };

        let ifaces = try!(final_ifaces.finalize(&ins_cursor.tree().children_names.clone()));
        let object = try!(Object::new(&path, ifaces));
        Ok(ins_cursor.set_object(object))

        // TODO: emit InterfacesAdded signal
    }

    pub fn remove(&mut self, path: &str) -> Result<Object, Error> {
        if !path.starts_with("/") {
            return Err(Error::InvalidPath(path.to_owned()));
        }

        let top_cursor = ObjectTreeCursor::new(self);

        path.split("/").skip(1).fold(Ok(top_cursor), |res_cursor, component| {
            res_cursor.and_then(|cursor| {
                if component.is_empty() {
                    return Err(Error::InvalidPath(path.to_owned()));
                }

                cursor.find(component)
                      .ok_or(Error::NoSuchPath(path.to_owned()))
            })
        }).and_then(|cursor| {
            cursor.remove(&path)
                  .ok_or(Error::NoSuchPath(path.to_owned()))
        })
    }
}

/// A representation of a collection of objects which implement an interface.
pub struct Server {
    conn: Rc<Connection>,
    name: String,
    can_handle: bool,

    objects: ObjectTree,
    signals: SignalHandlerMap,
    namespace_signals: SignalHandlerMap,
}

impl Server {
    /// Create a new `Server` to listen for signals.
    pub fn new_listener(conn: Rc<Connection>, name: &str) -> Result<Self, Error> {
        Ok(Server {
            conn: conn,
            name: name.to_owned(),
            can_handle: false,

            objects: ObjectTree::new(),
            signals: SignalHandlerMap::new(),
            namespace_signals: SignalHandlerMap::new(),
        })
    }

    /// Create a new `Server` to handle method calls from the bus.
    pub fn new(conn: Rc<Connection>, name: &str) -> Result<Self, Error> {
        try!(conn.request_name(name, DO_NOT_QUEUE));

        // TODO: Add match for the server.
        // TODO: add root object
        // TODO: add ObjectManager interface

        Ok(Server {
            conn: conn,
            name: name.to_owned(),
            can_handle: true,

            objects: ObjectTree::new(),
            signals: SignalHandlerMap::new(),
            namespace_signals: SignalHandlerMap::new(),
        })
    }

    /// The name of the server.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add an object to the server with the given interfaces.
    pub fn add_object(&mut self, path: &str, ifaces: InterfacesBuilder) -> Result<&mut Self, Error> {
        if !self.can_handle {
            return Err(Error::NoServerName);
        }

        self.objects.insert(path.to_owned(), ifaces)
            .map(|_| self)

    }

    pub fn add_object_manager(&mut self, path: &str, ifaces: InterfacesBuilder) -> Result<&mut Self, Error> {
        if !self.can_handle {
            return Err(Error::NoServerName);
        }

        try!(self.objects.borrow_mut().insert(path.to_string(), ifaces, true));

        Ok(self)
    }

    /// Remove an object from the server.
    pub fn remove_object(&mut self, path: &str) -> Result<&mut Self, Error> {
        if !self.can_handle {
            return Err(Error::NoServerName);
        }

        self.objects.remove(path)
            .map(|obj| {
                let iface_dict = self.objects.iface_dict();

                // TODO: emit InterfacesRemoved signal

                self
            })
    }

    /// Connect a handler to a specific object's signal.
    ///
    /// This will register a callback to listen to a specific object's signals.
    pub fn connect<F>(&mut self, signal: Target, callback: F) -> Result<&mut Self, Error>
        where F: FnMut(&Connection, &Target) -> () + 'static {
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
    pub fn connect_namespace<F>(&mut self, signal: Target, callback: F) -> Result<&mut Self, Error>
        where F: FnMut(&Connection, &Target) -> () + 'static {
        let dbus_match = format!("type='signal',interface='{}',path_namespace='{}',member='{}'",
                                 signal.interface,
                                 signal.object,
                                 signal.method);
        try!(self.conn.add_match(&dbus_match));

        _add_handler(&mut self.namespace_signals, signal, Rc::new(RefCell::new(callback)));

        Ok(self)
    }

    /// Handle a message with the appropriate handler.
    ///
    /// Returns `None` if the message was consumed, otherwise it returns the original message for
    /// further processing.
    pub fn handle_message<'b>(&self, m: &'b mut Message) -> Option<&'b mut Message> {
        match m.message_type() {
            MessageType::MethodCall => self._call_method(m),
            MessageType::Signal     => Some(self._match_signal(m)),
            _                       => Some(m),
        }
    }

    fn _call_method<'b>(&self, m: &'b mut Message) -> Option<&'b mut Message> {
        let conn = self.conn.clone();

        self.objects.iter().fold(Some(m), |opt_m, (_, object)| {
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

    fn _match_signal<'b>(&self, m: &'b mut Message) -> &'b mut Message {
        let conn = self.conn.clone();

        Target::extract(m).map(|signal| {
            for handlers in self.signals.get(&signal) {
                for handler in handlers.iter() {
                    let mut cb = handler.borrow_mut();

                    cb.deref_mut()(&conn, &signal);
                }
            }

            let matched_handlers = self.namespace_signals.iter().filter(|&(expect, _)| {
                expect.namespace_eq(&signal)
            });

            for (_, handlers) in matched_handlers {
                for handler in handlers.iter() {
                    let mut cb = handler.borrow_mut();

                    cb.deref_mut()(&conn, &signal);
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
