use super::connection::DBusConnection;
use super::error::DBusError;
use super::server::DBusServer;

use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

pub struct DBusRunner {
    conn: Rc<DBusConnection>,

    listeners: Vec<DBusServer>,
    servers: BTreeMap<String, DBusServer>,
}

impl DBusRunner {
    pub fn new(conn: DBusConnection) -> Result<DBusRunner, DBusError> {
        Ok(DBusRunner {
            conn: Rc::new(conn),

            listeners: vec![],
            servers: BTreeMap::new(),
        })
    }

    pub fn add_listener(&mut self, name: &str) -> Result<&mut DBusServer, DBusError> {
        let listener = try!(DBusServer::new_listener(self.conn.clone(), name));

        self.listeners.push(listener);

        Ok(self.listeners.last_mut().unwrap())
    }

    pub fn add_server(&mut self, name: &str) -> Result<&mut DBusServer, DBusError> {
        match self.servers.entry(name.to_owned()) {
            Entry::Vacant(v)    => {
                let server = try!(DBusServer::new(self.conn.clone(), name));

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

    pub fn run(&mut self) -> () {
        let listeners = &mut self.listeners;
        let servers = &mut self.servers;

        self.conn.iter().fold((), |_, mut message| {
            if message.is_signal() {
                for listener in listeners.iter_mut() {
                    listener.handle_message(&mut message);
                }
            }

            servers.iter_mut().fold(Some(&mut message), |opt_m, (_, server)| {
                opt_m.and_then(|m| {
                    server.handle_message(m)
                })
            });
        });
    }
}
