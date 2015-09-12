use super::message::DBusMessage;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DBusTarget {
    pub interface: String,
    pub object: String,
    pub method: String,
}

impl DBusTarget {
    pub fn new<I: ToString, O: ToString, M: ToString>(interface: I, object: O, method: M) -> DBusTarget {
        DBusTarget {
            interface: interface.to_string(),
            object: object.to_string(),
            method: method.to_string(),
        }
    }

    pub fn extract(m: &DBusMessage) -> Option<DBusTarget> {
        let (_, opt_interface, opt_object, opt_method) = m.headers();

        opt_interface.and_then(|interface| {
            opt_object.and_then(|object| {
                opt_method.map(|method| {
                    DBusTarget::new(interface, object, method)
                })
            })
        })
    }
}
