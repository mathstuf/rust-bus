extern crate dbus_bytestream;
use self::dbus_bytestream::message;

#[derive(Debug)]
pub struct DBusMessage {
    message: message::Message,
}
