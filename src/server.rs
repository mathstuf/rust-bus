extern crate log;

use super::connection::{DBusConnection, DBusReleaseNameReply, DBusRequestNameFlags};
use super::error::DBusError;
use super::interface::{DBusArgument, DBusMap, DBusChildrenList, DBusInterface, DBusInterfaceMap, DBusInterfaceMapBuilder, DBusMethod, DBusMethodResult};
use super::message::DBusMessage;
use super::object::DBusObject;
use super::target::DBusTarget;

use std::cell::{RefCell, RefMut};
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::{Rc, Weak};

pub type DBusSignalHandler = Box<FnMut(&DBusConnection, &DBusTarget) -> ()>;
type DBusSignalHandlers = Vec<DBusSignalHandler>;
type DBusSignalHandlerMap = BTreeMap<DBusTarget, DBusSignalHandlers>;

fn _add_handler(handlers: &mut DBusSignalHandlerMap, signal: DBusTarget, handler: DBusSignalHandler) {
    match handlers.entry(signal) {
        Entry::Vacant(v)    => { v.insert(vec![handler]); },
        Entry::Occupied(o)  => o.into_mut().push(handler),
    };
}

struct DBusObjectTreeCursor {
    tree: ObjectTree,
}

impl DBusObjectTreeCursor {
    pub fn new(tree: ObjectTree) -> Self {
        DBusObjectTreeCursor {
            tree: tree,
        }
    }

    pub fn tree(&self) -> &ObjectTree {
        &self.tree
    }

    pub fn has_object(&self) -> bool {
        self.tree.borrow().object.is_some()
    }

    pub fn is_manager(&self) -> bool {
        self.tree.borrow().manager
    }

    pub fn set_object(&mut self, object: DBusObject, manager: bool) {
        self.tree.borrow_mut().object = Some(object);
        self.tree.borrow_mut().manager = manager;
    }

    pub fn find_or_create(self, name: &str) -> Self {
        DBusObjectTreeCursor::new(self.tree.borrow_mut().find_or_create_object(name))
    }

    pub fn find(self, name: &str) -> Option<Self> {
        self.tree.borrow().find_object(name).map(DBusObjectTreeCursor::new)
    }

    pub fn remove(self, name: &str) -> Option<DBusObject> {
        self.tree.borrow_mut().remove_object(name)
    }
}

struct DBusObjectManagerInterface;

impl DBusObjectManagerInterface {
    fn get_managed_objects() -> DBusMethodResult {
        unimplemented!()
    }

    pub fn new() -> DBusInterface {
        DBusInterface::new()
            .add_method("GetManagedObjects", DBusMethod::new(move |_| Self::get_managed_objects())
                .add_result(DBusArgument::new("objpath_interfaces_and_properties", "a{oa{sa{sv}}}")))
    }
}

type ObjectTree = Rc<RefCell<DBusObjectTree>>;
type ObjectTreeRef = Weak<RefCell<DBusObjectTree>>;
type ObjectTreeMap = DBusMap<ObjectTree>;

struct DBusObjectTree {
    object: Option<DBusObject>,
    manager: bool,
    children_names: DBusChildrenList,
    children: ObjectTreeMap,
    self_ref: Option<ObjectTreeRef>,
}

impl DBusObjectTree {
    fn new_empty() -> Self {
        DBusObjectTree {
            object: None,
            manager: false,
            children_names: Rc::new(RefCell::new(vec![])),
            children: DBusMap::new(),
            self_ref: None,
        }
    }

    pub fn new() -> ObjectTree {
        let self_ref = Rc::new(RefCell::new(Self::new_empty()));

        RefMut::map(self_ref.borrow_mut(), |tree| {
            tree.self_ref = Some(Rc::downgrade(&self_ref));
            tree
        });

        self_ref
    }

    pub fn find_object(&self, name: &str) -> Option<Rc<RefCell<DBusObjectTree>>> {
        self.children.get(name).map(|o| {
            o.clone()
        })
    }

    pub fn find_or_create_object(&mut self, name: &str) -> ObjectTree {
        let children_mod = self.children_names.clone();

        self.children.entry(name.to_owned())
            .or_insert_with(move || {
                children_mod.borrow_mut().push(name.to_owned());
                DBusObjectTree::new()
            }).clone()
    }

    pub fn remove_object(&mut self, name: &str) -> Option<DBusObject> {
        let children_mod = self.children_names.clone();

        self.children.remove(name).and_then(|obj| {
            let pos = children_mod.borrow().iter().position(|child| child == name);
            children_mod.borrow_mut().swap_remove(pos.expect("child was not tracked?"));

            // TODO: emit InterfacesRemoved signals

            obj.borrow().object
        })
    }

    pub fn insert(&mut self, path: String, iface_map: DBusInterfaceMapBuilder, manager: bool) -> Result<(), DBusError> {
        if !path.starts_with("/") {
            return Err(DBusError::InvalidPath(path));
        }

        let top_cursor = DBusObjectTreeCursor::new(self.self_ref.unwrap().upgrade().unwrap());

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

        let final_iface = if manager {
            try!(iface_map.add_interface("org.freedesktop.DBus.ObjectManager", DBusObjectManagerInterface::new()))
        } else {
            iface_map
        };

        let rc_iface_map = Rc::new(try!(DBusInterfaceMap::new(iface_map, Rc::downgrade(&ins_cursor.tree().borrow().children_names))));
        Ok(ins_cursor.set_object(DBusObject::new(&path, rc_iface_map), manager))

        // TODO: emit InterfacesAdded signal
    }

    pub fn remove(&mut self, path: String) -> Result<DBusObject, DBusError> {
        if !path.starts_with("/") {
            return Err(DBusError::InvalidPath(path));
        }

        let full_path = path.clone();
        let mut cursor = DBusObjectTreeCursor::new(self.self_ref.unwrap().upgrade().unwrap().clone());

        let mut iter = path.split("/").skip(1).peekable();
        loop {
            match iter.peek() {
                Some(component) => {
                    if component.is_empty() {
                        return Err(DBusError::InvalidPath(full_path.clone()));
                    }

                    cursor = try!(cursor.find(component).ok_or(DBusError::NoSuchPath(full_path.clone())));
                },
                None            => break,
            }

            iter.next();
        }

        iter.next().and_then(|component| cursor.remove(component))
            .ok_or(DBusError::NoSuchPath(full_path))
    }
}

pub struct DBusServer {
    conn: Rc<DBusConnection>,
    name: String,
    can_handle: bool,

    objects: ObjectTree,
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

    pub fn add_object_manager<P: ToString>(&mut self, path: P, iface_map: DBusInterfaceMapBuilder) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        try!(self.objects.borrow_mut().insert(path.to_string(), iface_map, true));

        Ok(self)
    }

    pub fn remove_object<P: ToString>(&mut self, path: P) -> Result<&mut Self, DBusError> {
        if !self.can_handle {
            return Err(DBusError::NoServerName);
        }

        try!(self.objects.borrow_mut().remove(path.to_string()));

        Ok(self)
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
        let opt_path = m.path();

        opt_path.and_then(|path| {
            if let Some(tree) = self.objects.borrow().find_object(&path) {
                tree.borrow_mut().object.as_mut().and_then(|mut obj| {
                    match obj.handle_message(&conn, m) {
                        None          => Some(m),
                        Some(Ok(()))  => None,
                        Some(Err(())) => {
                            println!("failed to send a reply for {:?}", m);
                            None
                        },
                    }
                })
            } else {
                let errmsg = m.error_message("org.freedesktop.DBus.Error.UnknownObject")
                              .add_argument(&format!("unknown object: {}", path));
                if let Err(err) = conn.send(errmsg) {
                    error!("failed to send error reply: {}", err)
                }
                None
            }
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
