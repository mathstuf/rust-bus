use super::message::Message;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub interface: String,
    pub object: String,
    pub method: String,
}

struct SignalHeaders {
    interface: String,
    object: String,
    method: String,
}

impl Target {
    pub fn new<I: ToString, O: ToString, M: ToString>(interface: I, object: O, method: M) -> Self {
        Target {
            interface: interface.to_string(),
            object: object.to_string(),
            method: method.to_string(),
        }
    }

    pub fn extract(m: &Message) -> Option<Self> {
        SignalHeaders::new(m).map(|hdrs| {
            Self::new(hdrs.interface, hdrs.object, hdrs.method)
        })
    }

    pub fn namespace_eq(&self, t: &Self) -> bool {
        self.interface == t.interface &&
        self.method == t.method &&
        t.object.starts_with(&format!("{}/", self.object))
    }
}

impl SignalHeaders {
    pub fn new(m: &Message) -> Option<Self> {
        m.interface().and_then(|interface| {
            m.path().and_then(|object| {
                m.member().map(|method| {
                    SignalHeaders {
                        interface: interface,
                        object: object,
                        method: method,
                    }
                })
            })
        })
    }
}
