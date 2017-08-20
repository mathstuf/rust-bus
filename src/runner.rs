// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use super::connection::Connection;
use super::error::Error;
use super::message::MessageType;
use super::server::Server;

use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

/// An object to handle messages and act on them.
///
/// A `Runner` object listens to the message bus and handles them off to the appropriate objects
/// and signal handler callbacks.
pub struct Runner {
    conn: Rc<Connection>,

    listeners: Vec<Server>,
    servers: BTreeMap<String, Server>,
}

impl Runner {
    /// Create a new runner for the given connection.
    pub fn new(conn: Connection) -> Result<Self, Error> {
        Ok(Runner {
            conn: Rc::new(conn),

            listeners: vec![],
            servers: BTreeMap::new(),
        })
    }

    // FIXME: Rename to `new_listener`?
    /// Create a server which will listen for and handle signals.
    pub fn add_listener(&mut self, name: &str) -> Result<&mut Server, Error> {
        let listener = try!(Server::new_listener(self.conn.clone(), name));

        self.listeners.push(listener);

        Ok(self.listeners.last_mut().unwrap())
    }

    // FIXME: Rename to `new_server`?
    /// Create a server which will expose objects and interfaces to the bus.
    pub fn add_server(&mut self, name: &str) -> Result<&mut Server, Error> {
        match self.servers.entry(name.to_owned()) {
            Entry::Vacant(v) => {
                let server = try!(Server::new(self.conn.clone(), name));

                Ok(v.insert(server))
            },
            Entry::Occupied(_) => Err(Error::ServerAlreadyRegistered(name.to_owned())),
        }
    }

    /// Remove a server from the bus.
    pub fn remove_server(&mut self, name: &str) -> Result<&mut Self, Error> {
        match self.servers.remove(name) {
            Some(_) => Ok(self),
            None => Err(Error::NoSuchServer(name.to_owned())),
        }
    }

    // FIXME: Allow this to hook into other event loops.
    /// Run an event loop to handle messages.
    pub fn run(&mut self) -> () {
        let listeners = &mut self.listeners;
        let servers = &mut self.servers;

        // TODO: add dummy objects to servers

        self.conn.iter().fold((), |_, mut message| {
            if let MessageType::Signal = message.message_type() {
                for listener in listeners.iter_mut() {
                    listener.handle_message(&mut message);
                }
            }

            servers.iter_mut().fold(Some(&mut message), |opt_m, (_, server)| {
                opt_m.and_then(|m| server.handle_message(m))
            });
        });
    }
}
