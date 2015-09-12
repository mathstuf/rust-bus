extern crate dbus;
use self::dbus::ConnectionItem;

use super::connection::DBusConnection;
use super::error::DBusError;
use super::message::DBusMessageType;
use super::server::DBusServer;

use std::collections::btree_map::{BTreeMap, Entry};

pub struct DBusRunner<'a> {
    conn: &'a DBusConnection,

    listeners: Vec<DBusServer<'a>>,
    servers: BTreeMap<String, DBusServer<'a>>,
}

impl<'a> DBusRunner<'a> {
    pub fn new(conn: &'a DBusConnection) -> Result<DBusRunner<'a>, DBusError> {
        Ok(DBusRunner {
            conn: conn,

            listeners: vec![],
            servers: BTreeMap::new(),
        })
    }

    pub fn add_listener(&mut self, name: &str) -> Result<&mut DBusServer<'a>, DBusError> {
        let listener = try!(DBusServer::new_listener(&self.conn, name));

        self.listeners.push(listener);

        Ok(self.listeners.last_mut().unwrap())
    }

    pub fn add_server(&mut self, name: &str) -> Result<&mut DBusServer<'a>, DBusError> {
        match self.servers.entry(name.to_owned()) {
            Entry::Vacant(v)    => {
                let server = try!(DBusServer::new(&self.conn, name));

                Ok(v.insert(server))
            },
            Entry::Occupied(_)  => Err(DBusError::ServerAlreadyRegistered(name.to_owned())),
        }
    }

    pub fn remove_server(&mut self, name: &str) -> Result<&mut Self, DBusError> {
        match self.servers.remove(name) {
            Some(_) => Ok(self),
            None    => Err(DBusError::NoSuchServer(name.to_owned())),
        }
    }

    pub fn run(&mut self, timeout: i32) -> () {
        let listeners = &mut self.listeners;
        let servers = &mut self.servers;

        self.conn._connection().iter(timeout).fold((), |_, item| {
            match item {
                ConnectionItem::MethodCall(m) => Some(m),
                ConnectionItem::Signal(s)     => Some(s),
                ConnectionItem::Nothing       => None,
            }.as_mut().map(|m| {
                if m.msg_type() == DBusMessageType::Signal {
                    for listener in listeners.iter_mut() {
                        listener.handle_message(m);
                    }
                }

                servers.iter_mut().fold(Some(m), |opt_m, (_, server)| {
                    opt_m.and_then(|m| {
                        server.handle_message(m)
                    })
                })
            });
        });
    }
}
