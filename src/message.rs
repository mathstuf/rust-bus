extern crate dbus_bytestream;
use self::dbus_bytestream::message;

extern crate dbus_serialize;
use self::dbus_serialize::types::Variant;

use super::error::Error;
use super::value::{BasicValue, Marshal, Value};

#[derive(Debug)]
/// A message to communicate on the D-Bus.
pub struct Message {
    #[doc(hidden)]
    // This is used inside of the implementation, but should not be fully public.
    pub message: message::Message,
}

/// The type of a message.
pub enum MessageType {
    /// An error message.
    Error,
    /// A malformed message.
    Invalid,
    /// A message for a method call.
    MethodCall,
    /// A message for a return from a method call.
    MethodReturn,
    /// A signal.
    Signal,
}

impl Message {
    /// Create a new message from the underlying data type.
    pub fn new(message: message::Message) -> Self {
        Message {
            message: message,
        }
    }

    /// Create a call to a method.
    pub fn new_method_call(dest: &str, path: &str, iface: &str, method: &str) -> Self {
        Message {
            message: message::create_method_call(dest, path, iface, method),
        }
    }

    /// Create a signal message.
    pub fn new_signal(path: &str, iface: &str, method: &str) -> Self {
        Message {
            message: message::create_signal(path, iface, method),
        }
    }

    /// Create an error message.
    pub fn error_message(&self, name: &str) -> Self {
        Message {
            message: message::create_error(name, self.message.serial),
        }
    }

    /// Create a message which is a return value for the current message.
    ///
    /// This is used so that the return value is associated with the method call message.
    pub fn return_message(&self) -> Self {
        Message {
            message: message::create_method_return(self.message.serial),
        }
    }

    /// Add an argument to the message.
    pub fn add_argument(self, arg: &Marshal) -> Self {
        Message {
            message: self.message.add_arg(arg),
        }
    }

    /// The type of the message.
    pub fn message_type(&self) -> MessageType {
        match self.message.message_type {
            message::MESSAGE_TYPE_ERROR         => MessageType::Error,
            message::MESSAGE_TYPE_INVALID       => MessageType::Invalid,
            message::MESSAGE_TYPE_METHOD_CALL   => MessageType::MethodCall,
            message::MESSAGE_TYPE_METHOD_RETURN => MessageType::MethodReturn,
            message::MESSAGE_TYPE_SIGNAL        => MessageType::Signal,
            _                                   => MessageType::Invalid,
        }
    }

    fn _extract_string(v: &Variant) -> Option<String> {
        if let Value::BasicValue(BasicValue::String(ref s)) = *v.object {
            Some(s.clone())
        } else {
            None
        }
    }

    fn _get_header_string(message: &message::Message, header: u8) -> Option<String> {
        message.get_header(header)
            .and_then(Self::_extract_string)
    }

    /// The interface the message is destined for.
    pub fn interface(&self) -> Option<String> {
        Self::_get_header_string(&self.message, message::HEADER_FIELD_INTERFACE)
    }

    /// The object path the message is destined for.
    pub fn path(&self) -> Option<String> {
        Self::_get_header_string(&self.message, message::HEADER_FIELD_PATH)
    }

    /// The method or signal name the message is associated with.
    pub fn member(&self) -> Option<String> {
        Self::_get_header_string(&self.message, message::HEADER_FIELD_MEMBER)
    }

    /// Unpack the argument values stored within the message.
    pub fn values(&self) -> Result<Option<Vec<Value>>, Error> {
        Ok(try!(self.message.get_body()))
    }
}
