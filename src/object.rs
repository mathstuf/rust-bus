extern crate dbus;
use self::dbus::obj::ObjectPath;

use super::connection::DBusConnection;
use super::error::DBusError;
use super::interface::DBusInterfaceMap;
use super::message::DBusMessage;

pub struct DBusObject<'a> {
    name: String,
    path: ObjectPath<'a>,
}

impl<'a> DBusObject<'a> {
    pub fn new(conn: &'a DBusConnection, path: &str, iface_map: DBusInterfaceMap<'a>) -> Result<DBusObject<'a>, DBusError> {
        let mut obj = ObjectPath::new(conn, path, true);

        iface_map.into_iter().fold((), |_, (name, iface)| {
            obj.insert_interface(name, iface)
        });

        let mut dbusobj = DBusObject {
            name: path.to_owned(),
            path: obj,
        };
        try!(dbusobj._register());

        Ok(dbusobj)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn handle_message(&mut self, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        self.path.handle_message(msg)
    }

    fn _register(&mut self) -> Result<(), DBusError> {
        Ok(try!(self.path.set_registered(true)))
    }

    fn _unregister(&mut self) -> Result<(), DBusError> {
        Ok(try!(self.path.set_registered(false)))
    }
}

impl<'a> Drop for DBusObject<'a> {
    fn drop(&mut self) {
        let res = self._unregister();
        match res {
            Ok(_)    => (),
            Err(err) => println!("failed to release {}: {:?}", self.name, err),
        }
    }
}
