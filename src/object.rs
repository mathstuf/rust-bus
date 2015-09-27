use super::connection::DBusConnection;
use super::error::DBusError;
use super::interface::DBusInterfaceMap;
use super::message::DBusMessage;

pub struct DBusObject {
    path: String,

    interfaces: DBusInterfaceMap,
}

impl DBusObject {
    pub fn new(path: &str, interfaces: DBusInterfaceMap) -> Result<DBusObject, DBusError> {
        Ok(DBusObject {
            path: path.to_owned(),
            interfaces: try!(interfaces.finalize()),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handle_message(&mut self, conn: &DBusConnection, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        unimplemented!()
    }
}
