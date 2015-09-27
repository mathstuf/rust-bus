extern crate dbus_bytestream;
use self::dbus_bytestream::message;

use super::value::DBusMarshal;

#[derive(Debug)]
pub struct DBusMessage {
    message: message::Message,
}

pub type DBusHeaders = (String, String, String, String);

impl DBusMessage {
    pub fn new(message: message::Message) -> DBusMessage {
        DBusMessage {
            message: message,
        }
    }

    pub fn new_method_call(dest: &str, path: &str, iface: &str, method: &str) -> DBusMessage {
        DBusMessage {
            message: message::create_method_call(dest, path, iface, method),
        }
    }

    pub fn new_signal(path: &str, iface: &str, method: &str) -> DBusMessage {
        DBusMessage {
            message: message::create_signal(path, iface, method),
        }
    }

    pub fn error_message(&self, name: &str) -> DBusMessage {
        DBusMessage {
            message: message::create_error(name, self.message.serial),
        }
    }

    pub fn return_message(&self) -> DBusMessage {
        DBusMessage {
            message: message::create_method_return(self.message.serial),
        }
    }

    pub fn add_argument(self, arg: &DBusMarshal) -> DBusMessage {
        DBusMessage {
            message: self.message.add_arg(arg),
        }
    }

    pub fn is_signal(&self) -> bool {
        self.message.message_type == message::MESSAGE_TYPE_SIGNAL
    }

    pub fn is_method_call(&self) -> bool {
        self.message.message_type == message::MESSAGE_TYPE_METHOD_CALL
    }

    pub fn should_handle(&self) -> bool {
        self.is_signal() || self.is_method_call()
    }

    pub fn headers(&self) -> Option<DBusHeaders> {
        unimplemented!()
    }

    pub fn extract(self) -> message::Message {
        self.message
    }
}
