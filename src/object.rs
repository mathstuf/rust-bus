// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use connection::Connection;
use error::Error;
use interface::Interfaces;
use message::Message;

/// An object which may receive messages.
pub struct Object {
    path: String,

    interfaces: Interfaces,
}

impl Object {
    /// Create a new object with the given path, interfaces, and children.
    ///
    /// The list of children is managed by the object owning the object.
    pub fn new(path: &str, interfaces: Interfaces) -> Result<Self, Error> {
        Ok(Object {
            path: path.to_owned(),
            interfaces: interfaces,
        })
    }

    /// The path of the object on the bus.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Give a message to the object to handle.
    pub fn handle_message(&self, conn: &Connection, msg: &mut Message) -> Option<Result<(), ()>> {
        self.interfaces.handle(conn, msg)
    }
}
