use super::error::DBusError;
use super::interface::DBusInterfaceMap;

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
}
