use super::connection::DBusConnection;
use super::interface::DBusInterfaceMap;
use super::message::DBusMessage;

use std::rc::Rc;

pub struct DBusObject {
    path: String,

    interfaces: Rc<DBusInterfaceMap>,
}

impl DBusObject {
    pub fn new<P: ToString>(path: P, interfaces: Rc<DBusInterfaceMap>) -> Self {
        DBusObject {
            path: path.to_string(),
            interfaces: interfaces,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handle_message(&mut self, conn: &DBusConnection, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        self.interfaces.handle(conn, msg)
    }
}
