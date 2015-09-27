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

pub struct DBusAnnotation {
    name: String,
    value: String,
}
type DBusAnnotations = Vec<DBusAnnotation>;

pub struct DBusErrorMessage {
    name: String,
    message: String,
}

pub type DBusMethodResult = Result<Vec<DBusValue>, DBusErrorMessage>;
pub type DBusMethodHandler = Box<FnMut(&mut DBusMessage) -> DBusMethodResult>;

pub struct DBusMethod {
    in_args: Vec<DBusArgument>,
    out_args: Vec<DBusArgument>,
    cb: DBusMethodHandler,
    anns: DBusAnnotations,
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

pub struct DBusSignal {
    args: Vec<DBusArgument>,
    anns: DBusAnnotations,
}

pub struct DBusInterface {
    methods: DBusMap<DBusMethod>,
    properties: DBusMap<DBusProperty>,
    signals: DBusMap<DBusSignal>,
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
