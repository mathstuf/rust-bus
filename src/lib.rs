//! Module for exposing interfaces to a D-Bus manager.
//!
//! The `bus` crate makes it easy to expose interfaces on the D-Bus. Objects are created and added
//! to servers which delegate messages across the objects. The standard interfaces such as
//! properties and introspection are provided automatically.
//!
//! Servers may also be created to listen for signals and handle them.

#[macro_use]
extern crate bitflags;

mod arguments;
mod connection;
mod error;
mod interface;
mod message;
mod object;
mod runner;
mod server;
mod target;
mod value;

pub use connection::Connection;
pub use connection::ReleaseNameReply;
pub use connection::RequestNameFlags;
pub use connection::{ALLOW_REPLACEMENT, REPLACE_EXISTING, DO_NOT_QUEUE};
pub use connection::RequestNameReply;
pub use error::Error;
pub use interface::Annotation;
pub use interface::Argument;
pub use interface::ChildrenList;
pub use interface::ErrorMessage;
pub use interface::Interface;
pub use interface::Interfaces;
pub use interface::Method;
pub use interface::MethodHandler;
pub use interface::MethodResult;
pub use interface::Property;
pub use interface::PropertyReadHandler;
pub use interface::PropertyReadWriteHandler;
pub use interface::PropertyWriteHandler;
pub use interface::Signal;
pub use message::Message;
pub use message::MessageType;
pub use object::Object;
pub use runner::Runner;
pub use server::Server;
pub use target::Target;
pub use value::*;
