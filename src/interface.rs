use super::error::DBusError;
use super::message::DBusMessage;
use super::value::{DBusSignature, DBusValue};

use std::collections::btree_map::BTreeMap;

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

type InterfaceMap = DBusMap<DBusInterface>;

pub struct DBusInterfaceMap {
    map: InterfaceMap,
}

impl DBusInterfaceMap {
    pub fn finalize(mut self) -> Result<DBusInterfaceMap, DBusError> {
        unimplemented!()
    }
}
