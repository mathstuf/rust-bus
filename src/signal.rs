pub type DBusSignal = (String, String, String);

pub fn make_signal(interface: String, object: String, method: String) -> DBusSignal {
    (interface, object, method)
}
