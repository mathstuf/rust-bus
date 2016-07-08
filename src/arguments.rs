use super::interface::ErrorMessage;
use super::message::Message;
use super::value::{BasicValue, Value};

pub struct Arguments {
    values: Vec<Value>,
}

impl Arguments {
    pub fn new(msg: &Message) -> Result<Arguments, ErrorMessage> {
        Ok(Arguments {
            values: try!(msg.values().ok().and_then(|x| x).ok_or(Self::invalid_arguments())),
        })
    }

    pub fn extract(&self, index: usize) -> Result<&Value, ErrorMessage> {
        self.values.get(index).ok_or_else(|| Self::invalid_argument(index))
    }

    pub fn extract_string(&self, index: usize) -> Result<&String, ErrorMessage> {
        let value = try!(self.extract(index));
        if let Value::BasicValue(BasicValue::String(ref s)) = *value {
            Ok(s)
        } else {
            Err(Self::invalid_argument(index))
        }
    }

    pub fn invalid_arguments() -> ErrorMessage {
        ErrorMessage::new("org.freedesktop.DBus.Error.InvalidArgs", "invalid arguments")
    }

    fn invalid_argument(index: usize) -> ErrorMessage {
        ErrorMessage::new("org.freedesktop.DBus.Error.InvalidArgs", &format!("invalid argument at {}", index))
    }
}
