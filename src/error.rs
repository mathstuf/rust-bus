// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use crates::dbus_bytestream::connection;
use crates::dbus_bytestream::demarshal;

error_chain! {
    foreign_links {
        DBusMessage(connection::Error)
            #[doc = "An error message from the underlying D-Bus communication."];
    }

    errors {
        /// An invalid reply was received from a method call.
        InvalidReply(desc: String) {
            description("invalid reply")
            display("invalid reply: {}", desc)
        }

        /// An object was added to a signal-receiver server.
        NoServerName {
            description("listening server cannot handle methods")
        }

        /// A server with the given name was already registered.
        ServerAlreadyRegistered(name: String) {
            description("server already registered")
            display("server already registered: {}", name)
        }

        /// A request for a non-existent server was given.
        NoSuchServer(name: String) {
            description("no such server")
            display("no such server: {}", name)
        }

        /// An object was registered to a path, but it already existed.
        PathAlreadyRegistered(path: String) {
            description("path already registered")
            display("path already registered: {}", path)
        }

        /// An object was requested to be removed, but it does not exist.
        NoSuchPath(path: String) {
            description("no such path")
            display("no such path: {}", path)
        }

        /// Extracting values from a message body failed.
        ExtractArguments(err: demarshal::DemarshalError) {
            description("failed to extract arguments")
            display("failed to extract arguments: {}", err)
        }

        /// An attempt to redefine an interface for an object was made.
        InterfaceAlreadyRegistered(name: String) {
            description("interface already registered")
            display("interface already registered: {}", name)
        }
    }
}
