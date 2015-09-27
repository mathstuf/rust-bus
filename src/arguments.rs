use super::interface::DBusErrorMessage;
use super::message::DBusMessage;
use super::value::{DBusBasicValue, DBusValue};

pub struct DBusArguments {
    values: Vec<DBusValue>,
}

impl DBusArguments {
    pub fn new(msg: &DBusMessage) -> Result<Self, DBusErrorMessage> {
        Ok(DBusArguments {
            values: try!(msg.values().ok().and_then(|x| x).ok_or(Self::invalid_arguments())),
        })
    }

    pub fn extract(&self, index: usize) -> Result<&DBusValue, DBusErrorMessage> {
        self.values.get(index).ok_or(Self::invalid_argument(index))
    }

    pub fn extract_string(&self, index: usize) -> Result<&String, DBusErrorMessage> {
        let value = try!(self.extract(index));
        if let &DBusValue::BasicValue(DBusBasicValue::String(ref s)) = value {
            Ok(s)
        } else {
            Err(Self::invalid_argument(index))
        }
    }

    fn invalid_arguments() -> DBusErrorMessage {
        DBusErrorMessage::new("org.freedesktop.DBus.Error.InvalidArgs", "invalid arguments")
    }

    fn invalid_argument(index: usize) -> DBusErrorMessage {
        DBusErrorMessage::new("org.freedesktop.DBus.Error.InvalidArgs", &format!("invalid argument at {}", index))
    }
}
