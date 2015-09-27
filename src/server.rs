use super::connection::{DBusConnection, DBusReleaseNameReply, DBusRequestNameFlags};
use super::error::DBusError;
use super::interface::{DBusMap, DBusChildrenList, DBusInterfaceMap, DBusInterfaceMapBuilder};
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

struct DBusObjectTreeCursor<'a> {
    tree: &'a mut DBusObjectTree,
}

impl<'a> DBusObjectTreeCursor<'a> {
    pub fn new(tree: &'a mut DBusObjectTree) -> Self {
        DBusObjectTreeCursor {
            tree: tree,
        }
    }

    pub fn tree(&self) -> &DBusObjectTree {
        self.tree
    }

    pub fn has_object(&self) -> bool {
        self.tree.object.is_some()
    }

    pub fn set_object(&mut self, object: DBusObject) {
        self.tree.object = Some(object);
    }

    pub fn find_or_create(self, name: &str) -> Self {
        DBusObjectTreeCursor::new(self.tree.find_or_create_object(name))
    }

    pub fn find(self, name: &str) -> Option<Self> {
        self.tree.find_object(name).map(DBusObjectTreeCursor::new)
    }

    pub fn remove(self, name: &str) -> Option<DBusObject> {
        self.tree.remove_object(name)
    }
}

struct DBusObjectTree {
    object: Option<DBusObject>,
    children_names: DBusChildrenList,
    children: DBusMap<DBusObjectTree>,
}

impl DBusObjectTree {
    pub fn new() -> Self {
        DBusObjectTree {
            object: None,
            children_names: Rc::new(RefCell::new(vec![])),
            children: DBusMap::new(),
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
                DBusObjectTree::new()
            })
    }

    pub fn remove_object(&mut self, name: &str) -> Option<DBusObject> {
        //let children_mod = self.children_names.clone();

        self.children.remove(name).map(|obj| {
            unimplemented!()
        })
    }

    pub fn insert(&mut self, path: String, iface_map: DBusInterfaceMapBuilder) -> Result<(), DBusError> {
        if !path.starts_with("/") {
            return Err(DBusError::InvalidPath(path));
        }

        let top_cursor = DBusObjectTreeCursor::new(self);

        let mut ins_cursor = try!(path.split("/").skip(1).fold(Ok(top_cursor), |res_cursor, component| {
            res_cursor.and_then(|cursor| {
                if component.is_empty() {
                    return Err(DBusError::InvalidPath(path.clone()));
                }

                Ok(cursor.find_or_create(component))
            })
        }));

        if ins_cursor.has_object() {
            return Err(DBusError::PathAlreadyRegistered(path));
        }

        let iface_map = Rc::new(try!(DBusInterfaceMap::new(iface_map, ins_cursor.tree().children_names.clone())));
        Ok(ins_cursor.set_object(DBusObject::new(&path, iface_map)))

        // TODO: emit InterfacesAdded signal
    }

    pub fn remove(&mut self, path: String) -> Result<DBusObject, DBusError> {
        if !path.starts_with("/") {
            return Err(DBusError::InvalidPath(path));
        }

        unimplemented!()

        /*
        let full_path = path.clone();
        let top_cursor = DBusObjectTreeCursor::new(self);

        path.split("/").skip(1).fold(Ok(top_cursor), |res_cursor, component| {
            res_cursor.and_then(|cursor| {
                if component.is_empty() {
                    return Err(DBusError::InvalidPath(full_path.clone()));
                }

                cursor.find(component)
                    .ok_or(DBusError::NoSuchPath(full_path.clone()))
            })
        }).and_then(|cursor| {
            cursor.remove()
        }).ok_or(DBusError::NoSuchPath(full_path.clone()))
        */
    }
}

pub struct DBusServer {
    conn: Rc<DBusConnection>,
    name: String,
    can_handle: bool,

    objects: DBusObjectTree,
    signals: DBusSignalHandlerMap,
    namespace_signals: DBusSignalHandlerMap,
}

impl DBusServer {
    pub fn new_listener<N: ToString>(conn: Rc<DBusConnection>, name: N) -> Result<Self, DBusError> {
        Ok(DBusServer {
            conn: conn,
            name: name.to_string(),
            can_handle: false,

            objects: DBusObjectTree::new(),
            signals: DBusSignalHandlerMap::new(),
            namespace_signals: DBusSignalHandlerMap::new(),
        })
    }

    pub fn new<N: ToString>(conn: Rc<DBusConnection>, name: N) -> Result<Self, DBusError> {
        let name_str = name.to_string();
        try!(conn.request_name(&name_str, DBusRequestNameFlags::DoNotQueue));

        // TODO: add root object
        // TODO: add ObjectManager interface

        Ok(DBusServer {
            conn: conn,
            name: name_str,
            can_handle: true,

            objects: DBusObjectTree::new(),
            signals: DBusSignalHandlerMap::new(),
            namespace_signals: DBusSignalHandlerMap::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_object<P: ToString>(&mut self, path: P, iface_map: DBusInterfaceMapBuilder) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        try!(self.objects.borrow_mut().insert(path.to_string(), iface_map, false));

        Ok(self)
    }

    pub fn remove_object<P: ToString>(&mut self, path: P) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        unimplemented!()

        /*
        self.objects.remove(path.to_string())
            .map(|obj| {
                // TODO: emit InterfacesRemoved signal
                // TODO: add in dummy object

                self
            })
        */
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
        unimplemented!()
        /*
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
        */
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
