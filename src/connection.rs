extern crate dbus;
use self::dbus::{Connection, ConnectionItem};
pub use self::dbus::BusType as DBusBusType;

use super::error::DBusError;
use super::server::DBusServer;

use std::collections::btree_map::{BTreeMap, Entry};
use std::error::Error;

pub struct DBusConnection<'a> {
    conn: Connection,

    servers: BTreeMap<String, DBusServer<'a>>,
}

impl<'a> DBusConnection<'a> {
    pub fn new(bus: DBusBusType) -> Result<DBusConnection<'a>, dbus::Error> {
        let conn = try!(Connection::get_private(bus));

        Ok(DBusConnection {
            conn: conn,

            servers: BTreeMap::new(),
        })
    }

    pub fn add_server(&'a mut self, name: &str) -> Result<&mut DBusServer<'a>, Box<Error>> {
        match self.servers.entry(name.to_owned()) {
            Entry::Vacant(v)    => {
                let server = try!(DBusServer::new(&self.conn, name));

                Ok(v.insert(server))
            },
            Entry::Occupied(_)  => Err(Box::new(DBusError::ServerAlreadyRegistered(name.to_owned()))),
        }
    }

    pub fn remove_server(&mut self, name: &str) -> Result<&mut Self, DBusError> {
        match self.servers.remove(name) {
            Some(_) => Ok(self),
            None    => Err(DBusError::NoSuchServer(name.to_owned())),
        }
    }

    pub fn run(&mut self, timeout: i32) -> () {
        let servers = &mut self.servers;

        self.conn.iter(timeout).fold((), |_, item| {
            match item {
                ConnectionItem::MethodCall(m) => Some(m),
                ConnectionItem::Signal(s)     => Some(s),
                ConnectionItem::Nothing       => None,
            }.as_mut().map(|m| {
                servers.iter_mut().fold(Some(m), |opt_m, (_, server)| {
                    opt_m.and_then(|m| {
                        server.handle_message(m)
                    })
                })
            });
        });
    }
}
