use super::message::DBusMessage;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DBusTarget {
    pub interface: String,
    pub object: String,
    pub method: String,
}

impl DBusTarget {
    pub fn new<I: ToString, O: ToString, M: ToString>(interface: I, object: O, method: M) -> Self {
        DBusTarget {
            interface: interface.to_string(),
            object: object.to_string(),
            method: method.to_string(),
        }
    }

    pub fn extract(m: &DBusMessage) -> Option<Self> {
        m.signal_headers().map(|hdrs| {
            DBusTarget::new(hdrs.interface, hdrs.object, hdrs.method)
        })
    }

    pub fn namespace_eq(&self, t: &Self) -> bool {
        self.interface == t.interface &&
        self.method == t.method &&
        t.object.starts_with(&format!("{}/", self.object))
    }
}
