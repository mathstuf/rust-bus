// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

use super::message::Message;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
/// A representation of a signal which may be emitted.
pub struct Target {
    /// The interface the signal belongs to.
    pub interface: String,
    /// The object path which will emit the signal.
    pub object: String,
    /// The method name of the signal.
    pub method: String,
}

struct SignalHeaders {
    interface: String,
    object: String,
    method: String,
}

impl Target {
    /// Create a new `Target` structure.
    pub fn new<I: ToString, O: ToString, M: ToString>(interface: I, object: O, method: M) -> Self {
        Target {
            interface: interface.to_string(),
            object: object.to_string(),
            method: method.to_string(),
        }
    }

    /// Extract the signal from a `Message`.
    ///
    /// Returns `None` if parsing fails.
    pub fn extract(m: &Message) -> Option<Self> {
        SignalHeaders::new(m).map(|hdrs| Self::new(hdrs.interface, hdrs.object, hdrs.method))
    }

    /// Test if a `Target` matches this target.
    ///
    /// This is used to test if a signal belongs to the
    pub fn namespace_eq(&self, t: &Self) -> bool {
        self.interface == t.interface && self.method == t.method &&
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
