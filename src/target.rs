use super::message::Message;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub interface: String,
    pub object: String,
    pub method: String,
}

impl Target {
    pub fn new<I: ToString, O: ToString, M: ToString>(interface: I, object: O, method: M) -> Target {
        Target {
            interface: interface.to_string(),
            object: object.to_string(),
            method: method.to_string(),
        }
    }

    pub fn extract(m: &Message) -> Option<Target> {
        m.signal_headers().map(|hdrs| {
            Target::new(hdrs.interface, hdrs.object, hdrs.method)
        })
    }

    pub fn namespace_eq(&self, t: &Target) -> bool {
        self.interface == t.interface &&
        self.method == t.method &&
        t.object.starts_with(&format!("{}/", self.object))
    }
}
