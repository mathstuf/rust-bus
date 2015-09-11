extern crate dbus;
use self::dbus::Message;

pub type DBusTarget = (String, String, String);

pub fn make_target(interface: String, object: String, method: String) -> DBusTarget {
    (interface, object, method)
}

pub fn extract_target(m: &Message) -> Option<DBusTarget> {
    let (_, opt_interface, opt_object, opt_method) = m.headers();

    opt_interface.and_then(|interface| {
        opt_object.and_then(|object| {
            opt_method.map(|method| {
                (interface, object, method)
            })
        })
    })
}
