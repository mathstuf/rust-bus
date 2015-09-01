extern crate dbus;
use self::dbus::{Connection, Error, NameFlag, ReleaseNameReply};

pub struct DBusServer<'a> {
    conn: &'a Connection,
    name: String,
}

impl<'a> DBusServer<'a> {
    pub fn new(conn: &'a Connection, name: &str) -> Result<DBusServer<'a>, dbus::Error> {
        try!(conn.register_name(name, NameFlag::DoNotQueue as u32));

        Ok(DBusServer {
            conn: conn,
            name: name.to_string(),
        })
    }
}

impl<'a> Drop for DBusServer<'a> {
    fn drop(&mut self) {
        let res = self.conn.release_name(&self.name[..]);
        match res {
            Ok(reply) =>
                match reply {
                    ReleaseNameReply::Released    => (),
                    ReleaseNameReply::NonExistent => panic!("internal error: non-existent name {}?!", self.name),
                    ReleaseNameReply::NotOwner    => panic!("internal error: not the owner of {}?!", self.name),
                },
            Err(err) => println!("failed to release {}: {:?}: {:?}", self.name, err.name(), err.message()),
        }
    }
}
