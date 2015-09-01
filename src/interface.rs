extern crate dbus;
use self::dbus::{Connection, Error, Message, MessageItem};

pub trait DBusInterface {
    fn call_method(&mut self, method: &str, conn: &Connection, m: &Message) -> Option<Message>;

    fn set_property(&mut self, prop: &str, value: MessageItem) -> Result<(), Error>;
    fn get_property(&self, prop: &str) -> Result<(), Error>;
}
