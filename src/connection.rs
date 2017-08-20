// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use crates::dbus_bytestream::connection;

use error::*;
use message::{Message, MessageType};
use value::{BasicValue, Value};

bitflags! {
    /// Flags for use when requesting a name on the bus from the bus.
    pub flags RequestNameFlags: u32 {
        /// Allow replacement if another request for the same name later uses the
        /// `REPLACE_EXISTING` flag when requesting the same name.
        const ALLOW_REPLACEMENT = 0x1,
        /// Try and replace the service using the name requested, if one exists. It must have
        /// requested the name with `ALLOW_REPLACEMENT` for this to work.
        const REPLACE_EXISTING  = 0x2,
        /// By default, the request for the name will be placed into a queue to wait for the name
        /// to become available. Adding this flag will cause the request to fail instead.
        const DO_NOT_QUEUE      = 0x4,
    }
}

#[derive(Debug, PartialEq, Eq)]
/// Replies from the server when requesting a name.
pub enum RequestNameReply {
    /// The service has become the primary owner of the name.
    PrimaryOwner,
    /// The request is in the queue to become the owner of the name.
    InQueue,
    /// The name is already owned and may not be replaced.
    Exists,
    /// The application requesting the name already owns the name.
    AlreadyOwner,
}

#[derive(Debug, PartialEq, Eq)]
/// Replies from the server when releasing a name.
pub enum ReleaseNameReply {
    /// The name has been released.
    Released,
    /// The name is not bound to any service.
    NonExistent,
    /// The application releasing the name doesn't own the name.
    NotOwner,
}

/// An iterator over messages received from the message bus.
pub struct Messages<'a> {
    conn: &'a connection::Connection,
}

/// A connection to a bus.
///
/// A connection is usually to either the system bus or a session bus. User services (e.g.,
/// `SecretService`, notification daemons, etc.) live on the session bus while system services
/// (e.g., `Udisks2`, `NetworkManager`, etc.) live on the system bus.
pub struct Connection {
    conn: connection::Connection,
}

impl Connection {
    // TODO: Expose other connection methods?

    /// Connect to the session bus.
    pub fn session_new() -> Result<Self> {
        Ok(Connection {
            conn: connection::Connection::connect_session()?,
        })
    }

    /// Connect to the system bus.
    pub fn system_new() -> Result<Self> {
        Ok(Connection {
            conn: connection::Connection::connect_system()?,
        })
    }

    /// Request a name on the bus.
    ///
    /// By default, the name to address this connection directly is assigned by the daemon managing
    /// the bus, but a name for the application may be requested. Names are, by convention, in a
    /// reverse domain name format and use CamelCase for application-level names (e.g.,
    /// `com.example.Application`).
    pub fn request_name(&self, name: &str, flags: RequestNameFlags)
                        -> Result<RequestNameReply> {
        // TODO: Use an actual struct with an API for this.
        let msg = Message::new_method_call("org.freedesktop.DBus",
                                           "/org/freedesktop/DBus",
                                           "org.freedesktop.DBus",
                                           "RequestName")
            .add_argument(&name)
            .add_argument(&flags.bits);
        if let Some(mut results) = self.conn.call_sync(msg.message)? {
            if let Some(Value::BasicValue(BasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(RequestNameReply::PrimaryOwner),
                    2 => Ok(RequestNameReply::InQueue),
                    3 => Ok(RequestNameReply::Exists),
                    4 => Ok(RequestNameReply::AlreadyOwner),
                    _ => bail!(ErrorKind::InvalidReply(format!("RequestName: invalid response {}", r))),
                }
            } else {
                bail!(ErrorKind::InvalidReply("RequestName: invalid response".to_string()));
            }
        } else {
            bail!(ErrorKind::InvalidReply("RequestName: no response".to_string()));
        }
    }

    /// Release a name on the bus.
    pub fn release_name(&self, name: &str) -> Result<ReleaseNameReply> {
        // TODO: Use an actual struct with an API for this.
        let msg = Message::new_method_call("org.freedesktop.DBus",
                                           "/org/freedesktop/DBus",
                                           "org.freedesktop.DBus",
                                           "ReleaseName")
            .add_argument(&name);
        if let Some(mut results) = self.conn.call_sync(msg.message)? {
            if let Some(Value::BasicValue(BasicValue::Uint32(r))) = results.pop() {
                match r {
                    1 => Ok(ReleaseNameReply::Released),
                    2 => Ok(ReleaseNameReply::NonExistent),
                    3 => Ok(ReleaseNameReply::NotOwner),
                    _ => bail!(ErrorKind::InvalidReply(format!("ReleaseName: invalid response {}", r))),
                }
            } else {
                bail!(ErrorKind::InvalidReply("ReleaseName: invalid response".to_string()));
            }
        } else {
            bail!(ErrorKind::InvalidReply("ReleaseName: no response".to_string()));
        }
    }

    /// Requests the server to route messages to this connection.
    ///
    /// By default, the server will not deliver any messages to this connection. In order to
    /// receive messages, the manager must be told that the messages are wanted.
    ///
    /// The match syntax is documented in the [D-Bus
    /// specification](https://dbus.freedesktop.org/doc/dbus-specification.html#message-bus-routing).
    pub fn add_match(&self, match_rule: &str) -> Result<()> {
        let msg = Message::new_method_call("org.freedesktop.DBus",
                                           "/org/freedesktop/DBus",
                                           "org.freedesktop.DBus",
                                           "AddMatch")
            .add_argument(&match_rule);
        self.conn.call_sync(msg.message)?;
        Ok(())
    }

    /// Send a `Message` on the bus.
    ///
    /// On success, returns the serial number of the message.
    pub fn send(&self, msg: Message) -> Result<u32> {
        Ok(self.conn.send(msg.message)?)
    }

    /// An iterator over messages received over the bus.
    pub fn iter(&self) -> Messages {
        Messages {
            conn: &self.conn,
        }
    }
}

fn _should_handle(message: &Message) -> bool {
    match message.message_type() {
        MessageType::MethodCall | MessageType::Signal => true,
        _ => false,
    }
}

impl<'a> Iterator for Messages<'a> {
    type Item = Message;

    /// Returns messages received from the bus.
    ///
    /// Note that this currently blocks. See [this
    /// issue](https://github.com/srwalter/dbus-bytestream/issues/10) for progress on supporting an
    /// event loop.
    fn next(&mut self) -> Option<Self::Item> {
        let res = self.conn.read_msg();
        match res {
            Ok(message) => {
                let dbus_message = Message::new(message);
                if _should_handle(&dbus_message) {
                    Some(dbus_message)
                } else {
                    None
                }
            },
            Err(_) => None,
        }
    }
}
