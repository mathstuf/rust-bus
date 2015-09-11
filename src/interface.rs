extern crate dbus;
use self::dbus::{Connection, Error, Message, MessageItem};

pub enum DBusDirection {
    In,
    Out,
}

pub type DBusType = String;

pub struct DBusArgument {
    name: String,
    dbus_type: DBusType,

    direction: DBusDirection,
}

pub struct DBusMethod {
    name: String,
    args: Vec<DBusArgument>,
}

pub struct DBusSignal {
    name: String,
    args: Vec<DBusArgument>,
}

pub enum DBusAccess {
    Read,
    Write,
    ReadWrite,
}

pub struct DBusAnnotation {
    name: String,
    value: String,
}

pub struct DBusProperty {
    name: String,
    dbus_type: DBusType,
    access: DBusAccess,
    annotations: Vec<DBusAnnotation>,
}

pub trait DBusInterface {
    fn call_method(&mut self, method: &str, conn: &Connection, m: &Message) -> Option<Message>;

    fn set_property(&mut self, prop: &str, value: MessageItem) -> Result<(), Error>;
    fn get_property(&self, prop: &str) -> Result<(), Error>;

    fn methods(&self) -> &Vec<DBusMethod>;
    fn signals(&self) -> &Vec<DBusSignal>;
    fn properties(&self) -> &Vec<DBusProperty>;
}
