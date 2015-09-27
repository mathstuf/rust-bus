use super::connection::DBusConnection;
use super::error::DBusError;
use super::message::DBusMessage;
use super::value::{DBusSignature, DBusValue};

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

type DBusMap<T> = BTreeMap<String, T>;

pub struct DBusArgument {
    name: String,
    signature: String,
}

impl DBusArgument {
    pub fn new(name: &str, sig: &str) -> DBusArgument {
        DBusArgument {
            name: name.to_owned(),
            signature: sig.to_owned(),
        }
    }
}

pub struct DBusAnnotation {
    name: String,
    value: String,
}
type DBusAnnotations = Vec<DBusAnnotation>;

impl DBusAnnotation {
    pub fn new(name: &str, value: &str) -> DBusAnnotation {
        DBusAnnotation {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

pub struct DBusErrorMessage {
    name: String,
    message: String,
}

impl DBusErrorMessage {
    pub fn new(name: &str, message: &str) -> DBusErrorMessage {
        DBusErrorMessage {
            name: name.to_owned(),
            message: message.to_owned(),
        }
    }
}

pub type DBusMethodResult = Result<Vec<DBusValue>, DBusErrorMessage>;
pub type DBusMethodHandler = Box<FnMut(&mut DBusMessage) -> DBusMethodResult>;

pub struct DBusMethod {
    in_args: Vec<DBusArgument>,
    out_args: Vec<DBusArgument>,
    cb: DBusMethodHandler,
    anns: DBusAnnotations,
}

impl DBusMethod {
    pub fn new<F>(cb: F) -> DBusMethod
        where F: FnMut(&mut DBusMessage) -> DBusMethodResult + 'static {
        DBusMethod {
            in_args: vec![],
            out_args: vec![],
            cb: Box::new(cb),
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> DBusMethod {
        self.in_args.push(arg);

        self
    }

    pub fn add_result(mut self, arg: DBusArgument) -> DBusMethod {
        self.out_args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusMethod {
        self.anns.push(ann);

        self
    }
}

pub type DBusPropertyGetResult = Result<DBusValue, DBusErrorMessage>;
pub type DBusPropertySetResult = Result<(), DBusErrorMessage>;

pub trait DBusPropertyReadHandler {
    fn get(&self) -> DBusPropertyGetResult;
}

pub trait DBusPropertyWriteHandler {
    fn set(&self, &DBusValue) -> DBusPropertySetResult;
}

pub trait DBusPropertyReadWriteHandler {
    fn get(&self) -> DBusPropertyGetResult;
    fn set(&self, &DBusValue) -> DBusPropertySetResult;
}

enum PropertyAccess {
    RO(Box<DBusPropertyReadHandler>),
    RW(Box<DBusPropertyReadWriteHandler>),
    WO(Box<DBusPropertyWriteHandler>),
}

pub struct DBusProperty {
    signature: DBusSignature,
    access: PropertyAccess,
    anns: DBusAnnotations,
}

impl DBusProperty {
    fn new(sig: DBusSignature, access: PropertyAccess) -> DBusProperty {
        DBusProperty {
            signature: sig,
            access: access,
            anns: vec![],
        }
    }

    pub fn new_ro(sig: DBusSignature, access: Box<DBusPropertyReadHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::RO(access))
    }

    pub fn new_rw(sig: DBusSignature, access: Box<DBusPropertyReadWriteHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::RW(access))
    }

    pub fn new_wo(sig: DBusSignature, access: Box<DBusPropertyWriteHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::WO(access))
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusProperty {
        self.anns.push(ann);

        self
    }
}

pub struct DBusSignal {
    args: Vec<DBusArgument>,
    anns: DBusAnnotations,
}

impl DBusSignal {
    pub fn new() -> DBusSignal {
        DBusSignal {
            args: vec![],
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> DBusSignal {
        self.args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusSignal {
        self.anns.push(ann);

        self
    }
}

pub struct DBusInterface {
    methods: DBusMap<DBusMethod>,
    properties: DBusMap<DBusProperty>,
    signals: DBusMap<DBusSignal>,
}

impl DBusInterface {
    pub fn new() -> DBusInterface {
        DBusInterface {
            methods: BTreeMap::new(),
            properties: BTreeMap::new(),
            signals: BTreeMap::new(),
        }
    }

    pub fn add_method(mut self, name: &str, method: DBusMethod) -> DBusInterface {
        self.methods.insert(name.to_owned(), method);

        self
    }

    pub fn add_property(mut self, name: &str, property: DBusProperty) -> DBusInterface {
        self.properties.insert(name.to_owned(), property);

        self
    }

    pub fn get_property(&self, name: &str) -> Option<&DBusProperty> {
        self.properties.get(name)
    }

    pub fn add_signal(mut self, name: &str, signal: DBusSignal) -> DBusInterface {
        self.signals.insert(name.to_owned(), signal);

        self
    }
}

type InterfaceMap = Rc<RefCell<BTreeMap<String, DBusInterface>>>;

pub struct DBusInterfaceMap {
    map: InterfaceMap,
    finalized: bool,
}

impl DBusInterfaceMap {
    pub fn new() -> DBusInterfaceMap {
        DBusInterfaceMap {
            map: Rc::new(RefCell::new(BTreeMap::new())),
            finalized: false,
        }
    }

    // Marked as mut for intent; Rc<> doesn't require it though.
    #[allow(unused_mut)]
    pub fn add_interface(mut self, name: &str, iface: DBusInterface) -> Result<DBusInterfaceMap, DBusError> {
        if self.finalized {
            return Err(DBusError::InterfaceMapFinalized(name.to_owned()));
        }

        {
            let mut map = self.map.borrow_mut();

            match map.entry(name.to_owned()) {
                Entry::Vacant(v)    => {
                    v.insert(iface);

                    Ok(())
                },
                Entry::Occupied(_)  => Err(DBusError::InterfaceAlreadyRegistered(name.to_owned())),
            }
        }.map(|_| self)
    }

    pub fn finalize(mut self) -> Result<DBusInterfaceMap, DBusError> {
        // TODO: Add core interfaces.

        self.finalized = true;
        Ok(self)
    }

    pub fn handle(&self, conn: &DBusConnection, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        msg.call_headers().and_then(|hdrs| {
            let iface_name = hdrs.interface;
            let method_name = hdrs.method;
            self.map.borrow_mut().get_mut(&iface_name).and_then(|iface| iface.methods.get_mut(&method_name)).map(|method| {
                // TODO: Verify input argument signature.

                let msg = match (method.cb)(msg) {
                    Ok(vals) => {
                        vals.iter().fold(msg.return_message(), |msg, val| {
                            msg.add_argument(val)
                        })
                    },
                    Err(err) => msg.error_message(&err.name)
                                   .add_argument(&err.message),
                };

                // TODO: Verify that the signature matches the return.

                conn.send(msg)
                    .map(|_| ())
                    .map_err(|_| ())
            })
        })
    }
}
