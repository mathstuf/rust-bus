extern crate machine_id;
use self::machine_id::MachineId;

use super::arguments::DBusArguments;
use super::connection::DBusConnection;
use super::error::DBusError;
use super::message::DBusMessage;
use super::value::{DBusBasicValue, DBusSignature, DBusValue};

use std::cell::RefCell;
use std::collections::btree_map::{BTreeMap, Entry};
use std::rc::Rc;

type DBusMap<T> = BTreeMap<String, T>;

pub struct DBusArgument {
    name: String,
    signature: String,
}

impl DBusArgument {
    pub fn new(name: &str, sig: &str) -> DBusArgument {
        DBusArgument {
            name: name.to_owned(),
            signature: sig.to_owned(),
        }
    }
}

pub struct DBusAnnotation {
    name: String,
    value: String,
}
type DBusAnnotations = Vec<DBusAnnotation>;

impl DBusAnnotation {
    pub fn new(name: &str, value: &str) -> DBusAnnotation {
        DBusAnnotation {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

pub struct DBusErrorMessage {
    name: String,
    message: String,
}

impl DBusErrorMessage {
    pub fn new(name: &str, message: &str) -> DBusErrorMessage {
        DBusErrorMessage {
            name: name.to_owned(),
            message: message.to_owned(),
        }
    }
}

pub type DBusMethodResult = Result<Vec<DBusValue>, DBusErrorMessage>;
pub type DBusMethodHandler = Box<FnMut(&mut DBusMessage) -> DBusMethodResult>;

pub struct DBusMethod {
    in_args: Vec<DBusArgument>,
    out_args: Vec<DBusArgument>,
    cb: DBusMethodHandler,
    anns: DBusAnnotations,
}

impl DBusMethod {
    pub fn new<F>(cb: F) -> DBusMethod
        where F: FnMut(&mut DBusMessage) -> DBusMethodResult + 'static {
        DBusMethod {
            in_args: vec![],
            out_args: vec![],
            cb: Box::new(cb),
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> DBusMethod {
        self.in_args.push(arg);

        self
    }

    pub fn add_result(mut self, arg: DBusArgument) -> DBusMethod {
        self.out_args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusMethod {
        self.anns.push(ann);

        self
    }
}

pub type DBusPropertyGetResult = Result<DBusValue, DBusErrorMessage>;
pub type DBusPropertySetResult = Result<(), DBusErrorMessage>;

pub trait DBusPropertyReadHandler {
    fn get(&self) -> DBusPropertyGetResult;
}

pub trait DBusPropertyWriteHandler {
    fn set(&self, &DBusValue) -> DBusPropertySetResult;
}

pub trait DBusPropertyReadWriteHandler {
    fn get(&self) -> DBusPropertyGetResult;
    fn set(&self, &DBusValue) -> DBusPropertySetResult;
}

enum PropertyAccess {
    RO(Box<DBusPropertyReadHandler>),
    RW(Box<DBusPropertyReadWriteHandler>),
    WO(Box<DBusPropertyWriteHandler>),
}

pub struct DBusProperty {
    signature: DBusSignature,
    access: PropertyAccess,
    anns: DBusAnnotations,
}

impl DBusProperty {
    fn new(sig: DBusSignature, access: PropertyAccess) -> DBusProperty {
        DBusProperty {
            signature: sig,
            access: access,
            anns: vec![],
        }
    }

    pub fn new_ro(sig: DBusSignature, access: Box<DBusPropertyReadHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::RO(access))
    }

    pub fn new_rw(sig: DBusSignature, access: Box<DBusPropertyReadWriteHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::RW(access))
    }

    pub fn new_wo(sig: DBusSignature, access: Box<DBusPropertyWriteHandler>) -> DBusProperty {
        DBusProperty::new(sig, PropertyAccess::WO(access))
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusProperty {
        self.anns.push(ann);

        self
    }
}

pub struct DBusSignal {
    args: Vec<DBusArgument>,
    anns: DBusAnnotations,
}

impl DBusSignal {
    pub fn new() -> DBusSignal {
        DBusSignal {
            args: vec![],
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> DBusSignal {
        self.args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> DBusSignal {
        self.anns.push(ann);

        self
    }
}

pub struct DBusInterface {
    methods: DBusMap<DBusMethod>,
    properties: DBusMap<DBusProperty>,
    signals: DBusMap<DBusSignal>,
}

impl DBusInterface {
    pub fn new() -> DBusInterface {
        DBusInterface {
            methods: BTreeMap::new(),
            properties: BTreeMap::new(),
            signals: BTreeMap::new(),
        }
    }

    pub fn add_method(mut self, name: &str, method: DBusMethod) -> DBusInterface {
        self.methods.insert(name.to_owned(), method);

        self
    }

    pub fn add_property(mut self, name: &str, property: DBusProperty) -> DBusInterface {
        self.properties.insert(name.to_owned(), property);

        self
    }

    pub fn get_property(&self, name: &str) -> Option<&DBusProperty> {
        self.properties.get(name)
    }

    pub fn add_signal(mut self, name: &str, signal: DBusSignal) -> DBusInterface {
        self.signals.insert(name.to_owned(), signal);

        self
    }
}

type InterfaceMap = Rc<RefCell<BTreeMap<String, DBusInterface>>>;

pub struct DBusInterfaceMap {
    map: InterfaceMap,
    finalized: bool,
}

impl DBusInterfaceMap {
    pub fn new() -> DBusInterfaceMap {
        DBusInterfaceMap {
            map: Rc::new(RefCell::new(BTreeMap::new())),
            finalized: false,
        }
    }

    pub fn add_interface(mut self, name: &str, iface: DBusInterface) -> Result<DBusInterfaceMap, DBusError> {
        if self.finalized {
            return Err(DBusError::InterfaceMapFinalized(name.to_owned()));
        }

        {
            let mut map = self.map.borrow_mut();

            match map.entry(name.to_owned()) {
                Entry::Vacant(v)    => {
                    v.insert(iface);

                    Ok(())
                },
                Entry::Occupied(_)  => Err(DBusError::InterfaceAlreadyRegistered(name.to_owned())),
            }
        }.map(|_| self)
    }

    fn ping() -> Result<Vec<DBusValue>, DBusErrorMessage> {
        Ok(vec![])
    }

    fn get_machine_id() -> Result<Vec<DBusValue>, DBusErrorMessage> {
        let mid = format!("{}", MachineId::get());
        Ok(vec![DBusValue::BasicValue(DBusBasicValue::String(mid))])
    }

    fn get_property(map: &InterfaceMap, m: &mut DBusMessage) -> Result<Vec<DBusValue>, DBusErrorMessage> {
        let values = try!(m.values().ok_or(DBusArguments::invalid_arguments()));
        let iface = try!(DBusArguments::extract_string(&values, 0));
        let property = try!(DBusArguments::extract_string(&values, 1));

        match map.borrow().get(iface) {
            Some(iface) =>
                match iface.get_property(property) {
                    Some(prop) =>
                        match prop.access {
                            PropertyAccess::RO(ref ro) => Ok(vec![try!(ro.get())]),
                            PropertyAccess::RW(ref rw) => Ok(vec![try!(rw.get())]),
                            PropertyAccess::WO(_) =>
                                Err(DBusErrorMessage {
                                    name: "org.freedesktop.DBus.Error.Failed".to_owned(),
                                    message: format!("property is write-only: {}", property),
                                }),
                        },
                    None       =>
                        Err(DBusErrorMessage {
                            name: "org.freedesktop.DBus.Error.UnknownProperty".to_owned(),
                            message: format!("unknown property: {}", property),
                        }),
                },
            None        =>
                Err(DBusErrorMessage {
                    name: "org.freedesktop.DBus.Error.UnknownInterface".to_owned(),
                    message: format!("unknown interface: {}", iface),
                }),
        }
    }

    fn set_property(map: &mut InterfaceMap, m: &mut DBusMessage) -> Result<Vec<DBusValue>, DBusErrorMessage> {
        let values = try!(m.values().ok_or(DBusArguments::invalid_arguments()));
        let iface = try!(DBusArguments::extract_string(&values, 0));
        let property = try!(DBusArguments::extract_string(&values, 1));
        let value = try!(DBusArguments::extract(&values, 2));

        match map.borrow_mut().get(iface) {
            Some(iface) =>
                match iface.get_property(property) {
                    Some(prop) =>
                        match prop.access {
                            PropertyAccess::WO(ref wo) => {
                                try!(wo.set(value));
                                Ok(vec![])
                            },
                            PropertyAccess::RW(ref rw) => {
                                try!(rw.set(value));
                                Ok(vec![])
                            },
                            PropertyAccess::RO(_) =>
                                Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.Failed",
                                                          &format!("property is read-only: {}", property))),
                        },
                    None       =>
                        Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.UnknownProperty",
                                                  &format!("unknown property: {}", property))),
                },
            None        =>
                Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.UnknownInterface",
                                          &format!("unknown interface: {}", iface))),
        }
    }

    fn get_all_properties(map: &InterfaceMap, m: &mut DBusMessage) -> Result<Vec<DBusValue>, DBusErrorMessage> {
        let values = try!(m.values().ok_or(DBusArguments::invalid_arguments()));
        let iface = try!(DBusArguments::extract_string(&values, 0));

        match map.borrow().get(iface) {
            Some(iface) =>
                // TODO: implement
                Ok(vec![]),
            None        =>
                Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.UnknownInterface",
                                          &format!("unknown interface: {}", iface))),
        }
    }

    fn introspect(map: &InterfaceMap, _: &mut DBusMessage) -> Result<Vec<DBusValue>, DBusErrorMessage> {
        let xml = format!(concat!(
            r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"\n"#,
            r#" "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">\n"#,
            r#"<node>\n"#,
            r#"{}"#,
            // TODO: get child objects into here.
            r#"</node>\n"#),
            Self::_to_string_map(&*map.borrow(), |k, v| Self::_introspect_interface(" ", k, v)));
        Ok(vec![DBusValue::BasicValue(DBusBasicValue::String(xml))])
    }

    fn _to_string_map<K, V, F>(map: &BTreeMap<K, V>, f: F) -> String
        where F: Fn(&K, &V) -> String {
        map.iter().fold("".to_owned(), |p, (k, v)| {
            format!("{}{}", p, f(k, v))
        })
    }

    fn _to_string_list<T, F>(map: &Vec<T>, f: F) -> String
        where F: Fn(&T) -> String {
        map.iter().fold("".to_owned(), |p, t| {
            format!("{}{}", p, f(t))
        })
    }

    fn _introspect_annotation(indent: &str, ann: &DBusAnnotation) -> String {
        format!(r#"{}<annotation name="{}" value="{}" />\n"#,
            indent,
            ann.name,
            ann.value)
    }

    fn _introspect_arg(indent: &str, direction: &str, arg: &DBusArgument) -> String {
        format!(r#"{}<arg name="{}" type="{}" direction="{}" />\n"#,
            indent,
            arg.name,
            arg.signature,
            direction)
    }

    fn _introspect_property(indent: &str, name: &String, prop: &DBusProperty) -> String {
        let new_indent = format!("{} ", indent);
        let access =
            match prop.access {
                PropertyAccess::RO(_) => "read",
                PropertyAccess::RW(_) => "readwrite",
                PropertyAccess::WO(_) => "write",
            };
        let sig = match prop.signature { DBusSignature(ref s) => s };
        format!(r#"{}<property name="" type="{}" access="{}">\n{}{}</property>\n"#,
            name,
            sig,
            access,
            Self::_to_string_list(&prop.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_method(indent: &str, name: &String, method: &DBusMethod) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<method name="">\n{}{}{}{}</method>\n"#,
            name,
            Self::_to_string_list(&method.in_args, |t| Self::_introspect_arg(&new_indent, "in", t)),
            Self::_to_string_list(&method.out_args, |t| Self::_introspect_arg(&new_indent, "out", t)),
            Self::_to_string_list(&method.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_signal(indent: &str, name: &String, signal: &DBusSignal) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<signal name="">\n{}{}{}</signal>\n"#,
            name,
            Self::_to_string_list(&signal.args, |t| Self::_introspect_arg(&new_indent, "out", t)),
            Self::_to_string_list(&signal.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_interface(indent: &str, name: &String, iface: &DBusInterface) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<interface name="{}">\n{}{}{}{}</interface>\n"#,
            indent,
            name,
            Self::_to_string_map(&iface.properties, |k, v| Self::_introspect_property(&new_indent, k, v)),
            Self::_to_string_map(&iface.methods, |k, v| Self::_introspect_method(&new_indent, k, v)),
            Self::_to_string_map(&iface.signals, |k, v| Self::_introspect_signal(&new_indent, k, v)),
            indent)
    }

    pub fn finalize(mut self) -> Result<DBusInterfaceMap, DBusError> {
        if self.finalized {
            return Err(DBusError::InterfaceMapFinalized("org.freedesktop.DBus.Introspectable".to_owned()));
        }

        self = try!(self.add_interface("org.freedesktop.DBus.Peer", DBusInterface::new()
            .add_method("Ping", DBusMethod::new(|_| Self::ping()))
            .add_method("GetMachineId", DBusMethod::new(|_| Self::get_machine_id())
                .add_result(DBusArgument::new("machine_uuid", "s")))
        ));

        let get_map = self.map.clone();
        let mut set_map = self.map.clone();
        let get_all_map = self.map.clone();

        self = try!(self.add_interface("org.freedesktop.DBus.Properties", DBusInterface::new()
            .add_method("Get", DBusMethod::new(move |m| Self::get_property(&get_map, m))
                .add_argument(DBusArgument::new("interface_name", "s"))
                .add_argument(DBusArgument::new("property_name", "s"))
                .add_result(DBusArgument::new("value", "v")))
            .add_method("Set", DBusMethod::new(move |m| Self::set_property(&mut set_map, m))
                .add_argument(DBusArgument::new("interface_name", "s"))
                .add_argument(DBusArgument::new("property_name", "s"))
                .add_result(DBusArgument::new("value", "v")))
            .add_method("GetAll", DBusMethod::new(move |m| Self::get_all_properties(&get_all_map, m))
                .add_argument(DBusArgument::new("interface_name", "s"))
                .add_result(DBusArgument::new("props", "{sv}")))
        ));

        let introspect_map = self.map.clone();

        self = try!(self.add_interface("org.freedesktop.DBus.Introspectable", DBusInterface::new()
            .add_method("Introspect", DBusMethod::new(move |m| Self::introspect(&introspect_map, m))
                .add_result(DBusArgument::new("xml_data", "s")))
        ));

        self.finalized = true;
        Ok(self)
    }

    pub fn handle(&self, conn: &DBusConnection, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        // TODO: implement
        None
    }
}
