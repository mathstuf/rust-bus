extern crate log;

extern crate machine_id;
use self::machine_id::MachineId;

use super::arguments::DBusArguments;
use super::connection::DBusConnection;
use super::error::DBusError;
use super::message::DBusMessage;
use super::value::{DBusBasicValue, DBusDictionary, DBusSignature, DBusValue};

use std::cell::{Ref, RefCell};
use std::collections::btree_map::{BTreeMap, Entry};
use std::collections::HashMap;
use std::rc::Rc;

pub type DBusMap<T> = BTreeMap<String, T>;

pub struct DBusArgument {
    name: String,
    signature: String,
}

impl DBusArgument {
    pub fn new<N: ToString, S: ToString>(name: N, sig: S) -> Self {
        DBusArgument {
            name: name.to_string(),
            signature: sig.to_string(),
        }
    }
}

pub struct DBusAnnotation {
    name: String,
    value: String,
}
type DBusAnnotations = Vec<DBusAnnotation>;

impl DBusAnnotation {
    pub fn new<N: ToString, V: ToString>(name: N, value: V) -> Self {
        DBusAnnotation {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

pub struct DBusErrorMessage {
    name: String,
    message: String,
}

impl DBusErrorMessage {
    pub fn new<N: ToString, M: ToString>(name: N, message: M) -> Self {
        DBusErrorMessage {
            name: name.to_string(),
            message: message.to_string(),
        }
    }
}

pub type DBusMethodResult = Result<Vec<DBusValue>, DBusErrorMessage>;
pub type DBusMethodHandler = Rc<RefCell<FnMut(&mut DBusMessage) -> DBusMethodResult>>;

pub struct DBusMethod {
    in_args: Vec<DBusArgument>,
    out_args: Vec<DBusArgument>,
    cb: DBusMethodHandler,
    anns: DBusAnnotations,
}

fn _get_signature(args: &Vec<DBusArgument>) -> String {
    args.iter().fold("".to_owned(), |mut s, a| {
        s.push_str(&a.signature);
        s
    })
}

impl DBusMethod {
    pub fn new<F>(cb: F) -> Self
        where F: FnMut(&mut DBusMessage) -> DBusMethodResult + 'static {
        DBusMethod {
            in_args: vec![],
            out_args: vec![],
            cb: Rc::new(RefCell::new(cb)),
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> Self {
        self.in_args.push(arg);

        self
    }

    pub fn add_result(mut self, arg: DBusArgument) -> Self {
        self.out_args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> Self {
        self.anns.push(ann);

        self
    }

    pub fn signature(&self) -> String {
        _get_signature(&self.in_args)
    }

    pub fn result_signature(&self) -> String {
        _get_signature(&self.out_args)
    }

    pub fn call(&self, msg: &mut DBusMessage) -> DBusMethodResult {
        let mut f = self.cb.borrow_mut();
        (&mut *f)(msg)
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
    fn new(sig: DBusSignature, access: PropertyAccess) -> Self {
        DBusProperty {
            signature: sig,
            access: access,
            anns: vec![],
        }
    }

    pub fn new_ro(sig: DBusSignature, access: Box<DBusPropertyReadHandler>) -> Self {
        Self::new(sig, PropertyAccess::RO(access))
    }

    pub fn new_rw(sig: DBusSignature, access: Box<DBusPropertyReadWriteHandler>) -> Self {
        Self::new(sig, PropertyAccess::RW(access))
    }

    pub fn new_wo(sig: DBusSignature, access: Box<DBusPropertyWriteHandler>) -> Self {
        Self::new(sig, PropertyAccess::WO(access))
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> Self {
        self.anns.push(ann);

        self
    }
}

pub struct DBusSignal {
    args: Vec<DBusArgument>,
    anns: DBusAnnotations,
}

impl DBusSignal {
    pub fn new() -> Self {
        DBusSignal {
            args: vec![],
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: DBusArgument) -> Self {
        self.args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: DBusAnnotation) -> Self {
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
    pub fn new() -> Self {
        DBusInterface {
            methods: DBusMap::new(),
            properties: DBusMap::new(),
            signals: DBusMap::new(),
        }
    }

    pub fn add_method<N: ToString>(mut self, name: N, method: DBusMethod) -> Self {
        self.methods.insert(name.to_string(), method);

        self
    }

    pub fn add_property<N: ToString>(mut self, name: N, property: DBusProperty) -> Self {
        self.properties.insert(name.to_string(), property);

        self
    }

    pub fn get_property<N: ToString>(&self, name: N) -> Option<&DBusProperty> {
        self.properties.get(&name.to_string())
    }

    pub fn add_signal<N: ToString>(mut self, name: N, signal: DBusSignal) -> Self {
        self.signals.insert(name.to_string(), signal);

        self
    }

    fn _require_property(&self, name: &str) -> Result<&DBusProperty, DBusErrorMessage> {
        self.properties.get(name).ok_or(
            DBusErrorMessage::new("org.freedesktop.DBus.Error.UnknownProperty",
                                  &format!("unknown property: {}", name)))
    }

    pub fn get_property_value(&self, name: &str) -> DBusMethodResult {
        self._require_property(name).and_then(|prop| {
            match prop.access {
                PropertyAccess::RO(ref ro) => ro.get().map(|v| vec![v]),
                PropertyAccess::RW(ref rw) => rw.get().map(|v| vec![v]),
                PropertyAccess::WO(_) =>
                    Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.Failed",
                                              format!("property is write-only: {}", name))),
            }
        })
    }

    pub fn set_property_value(&self, name: &str, value: &DBusValue) -> DBusMethodResult {
        self._require_property(name).and_then(|prop| {
            match prop.access {
                PropertyAccess::WO(ref wo) => wo.set(value).map(|_| vec![]),
                PropertyAccess::RW(ref rw) => rw.set(value).map(|_| vec![]),
                PropertyAccess::RO(_) =>
                    Err(DBusErrorMessage::new("org.freedesktop.DBus.Error.Failed",
                                              &format!("property is read-only: {}", name))),
            }
        })
    }

    pub fn get_property_map(&self) -> DBusDictionary {
        DBusDictionary::new(self.properties.iter().map(|(k, v)| {
            match v.access {
                // TODO: Message that failures occurred?
                PropertyAccess::RO(ref ro) => ro.get().ok(),
                PropertyAccess::RW(ref rw) => rw.get().ok(),
                PropertyAccess::WO(_)      => None,
            }.map(|v| {
                (DBusBasicValue::String(k.clone()), v)
            })
        }).filter_map(|a| a).collect::<HashMap<DBusBasicValue, DBusValue>>())
    }
}

struct DBusPeerInterface;

impl DBusPeerInterface {
    fn ping() -> DBusMethodResult {
        Ok(vec![])
    }

    fn get_machine_id() -> DBusMethodResult {
        let mid = format!("{}", MachineId::get());
        Ok(vec![DBusValue::BasicValue(DBusBasicValue::String(mid))])
    }

    pub fn new() -> DBusInterface {
        DBusInterface::new()
            .add_method("Ping", DBusMethod::new(|_| Self::ping()))
            .add_method("GetMachineId", DBusMethod::new(|_| Self::get_machine_id())
                .add_result(DBusArgument::new("machine_uuid", "s")))
    }
}

struct DBusPropertyInterface;

impl DBusPropertyInterface {
    fn get_property(map: &InterfaceMap, m: &mut DBusMessage) -> DBusMethodResult {
        let values = try!(DBusArguments::new(m));
        let iface = try!(values.extract_string(0));
        let property = try!(values.extract_string(1));

        require_interface(&map.borrow(), iface).and_then(|iface| {
            iface.get_property_value(property)
        })
    }

    fn set_property(map: &mut InterfaceMap, m: &mut DBusMessage) -> DBusMethodResult {
        let values = try!(DBusArguments::new(m));
        let iface = try!(values.extract_string(0));
        let property = try!(values.extract_string(1));
        let value = try!(values.extract(2));

        require_interface(&map.borrow(), iface).and_then(|iface| {
            iface.set_property_value(property, value)
        })
    }

    fn get_all_properties(map: &InterfaceMap, m: &mut DBusMessage) -> DBusMethodResult {
        let values = try!(DBusArguments::new(m));
        let iface = try!(values.extract_string(0));

        require_interface(&map.borrow(), iface).map(|iface| {
            vec![DBusValue::Dictionary(iface.get_property_map())]
        })
    }

    pub fn new(map: InterfaceMap) -> DBusInterface {
        let get_map = map.clone();
        let mut set_map = map.clone();
        let get_all_map = map.clone();

        DBusInterface::new()
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
    }
}

struct DBusIntrospectableInterface;
pub type DBusChildrenList = Rc<RefCell<Vec<String>>>;

impl DBusIntrospectableInterface {
    fn introspect(map: &InterfaceMap, children: &DBusChildrenList) -> DBusMethodResult {
        let xml = format!(concat!(
            r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"\n"#,
            r#" "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">\n"#,
            r#"<!-- Rust EZ-DBus {} -->"#,
            r#"<node>\n"#,
            r#"{}"#, // interface
            r#"{}"#, // children
            r#"</node>\n"#),
            env!("CARGO_PKG_VERSION"),
            Self::_to_string_map(&*map.borrow(), |k, v| Self::_introspect_interface(" ", k, v)),
            children.borrow().iter().fold("".to_owned(), |p, name| {
                format!(r#"{} <node name="{}" />"#, p, name)
            }));
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

    pub fn new(map: InterfaceMap, children: DBusChildrenList) -> DBusInterface {
        let introspect_map = map.clone();
        let children = children.clone();

        DBusInterface::new()
            .add_method("Introspect", DBusMethod::new(move |_| Self::introspect(&introspect_map, &children))
                .add_result(DBusArgument::new("xml_data", "s")))
    }
}

fn require_interface<'a>(map: &'a Ref<'a, DBusMap<DBusInterface>>, name: &str) -> Result<&'a DBusInterface, DBusErrorMessage> {
    map.get(name).ok_or(
        DBusErrorMessage {
            name: "org.freedesktop.DBus.Error.UnknownInterface".to_owned(),
            message: format!("unknown interface: {}", name),
        })
}

type InterfaceMap = Rc<RefCell<DBusMap<DBusInterface>>>;

pub struct DBusInterfaceMapBuilder {
    map: DBusMap<DBusInterface>,
}
pub struct DBusInterfaceMap {
    map: InterfaceMap,
}

impl DBusInterfaceMapBuilder {
    pub fn new() -> Self {
        DBusInterfaceMapBuilder {
            map: DBusMap::new(),
        }
    }

    pub fn add_interface<N: ToString>(mut self, name: N, iface: DBusInterface) -> Result<Self, DBusError> {
        match self.map.entry(name.to_string()) {
            Entry::Vacant(v)    => {
                v.insert(iface);

                Ok(())
            },
            Entry::Occupied(_)  => Err(DBusError::InterfaceAlreadyRegistered(name.to_string())),
        }.map(|_| self)
    }
}

impl DBusInterfaceMap {
    pub fn new(builder: DBusInterfaceMapBuilder, children: DBusChildrenList) -> Result<Self, DBusError> {
        let this = DBusInterfaceMap {
            map: Rc::new(RefCell::new(builder.map)),
        };

        Ok(this)
            .and_then(|this| {
                this.add_interface("org.freedesktop.DBus.Peer", DBusPeerInterface::new())
            }).and_then(|this| {
                let property_map = this.map.clone();
                this.add_interface("org.freedesktop.DBus.Properties", DBusPropertyInterface::new(property_map))
            }).and_then(|this| {
                let introspectable_map = this.map.clone();
                this.add_interface("org.freedesktop.DBus.Introspectable", DBusIntrospectableInterface::new(introspectable_map, children))
            })
    }

    fn add_interface(self, name: &str, iface: DBusInterface) -> Result<Self, DBusError> {
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

    pub fn get_interfaces_and_properties(&self) -> DBusDictionary {
        DBusDictionary::new(self.map.borrow().iter().map(|(k, v)| {
            (DBusBasicValue::String(k.clone()), DBusValue::Dictionary(v.get_property_map()))
        }).collect::<HashMap<DBusBasicValue, DBusValue>>())
    }

    pub fn handle(&self, conn: &DBusConnection, msg: &mut DBusMessage) -> Option<Result<(), ()>> {
        msg.call_headers().map(|hdrs| {
            let iface_name = hdrs.interface;
            let method_name = hdrs.method;
            let reply = if let Some(iface) = self.map.borrow().get(&iface_name) {
                if let Some(method) = iface.methods.get(&method_name) {
                    let expect_sig = method.signature();
                    let actual_sig = msg.signature();
                    if actual_sig != expect_sig {
                        msg.error_message("org.freedesktop.DBus.Error.InvalidArgs")
                           .add_argument(&format!("invalid arguments: expected '{}'; received '{}'",
                                         expect_sig, actual_sig))
                    } else {
                        match method.call(msg) {
                            Ok(vals) => {
                                let ret = vals.iter().fold(msg.return_message(), |msg, val| {
                                    msg.add_argument(val)
                                });

                                let expect_ret_sig = method.result_signature();
                                let actual_ret_sig = ret.signature();
                                if actual_sig != expect_sig {
                                    warn!("invalid return signature: expected '{}'; received '{}'",
                                          expect_ret_sig, actual_ret_sig);
                                }

                                ret
                            },
                            Err(err) => msg.error_message(&err.name)
                                           .add_argument(&err.message),
                        }
                    }
                } else {
                    msg.error_message("org.freedesktop.DBus.Error.UnknownMethod")
                       .add_argument(&format!("unknown method: {}", method_name))
                }
            } else {
                msg.error_message("org.freedesktop.DBus.Error.UnknownMethod")
                   .add_argument(&format!("unknown interface: {}", iface_name))
            };

            conn.send(reply)
                .map(|_| ())
                .map_err(|err| {
                    warn!("failed to send error reply: {}", err)
                })
        })
    }
}
