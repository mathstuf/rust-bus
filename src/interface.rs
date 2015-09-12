extern crate dbus;

use std::collections::btree_map::BTreeMap;

pub use self::dbus::obj::Interface as DBusInterface;
pub type DBusInterfaceMap<'a> = BTreeMap<String, DBusInterface<'a>>;
