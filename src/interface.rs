extern crate core;
use self::core::ops::DerefMut;

extern crate machine_id;
use self::machine_id::MachineId;

use super::arguments::Arguments;
use super::connection::Connection;
use super::error::Error;
use super::message::{Message, MessageType};
use super::value::{BasicValue, Dictionary, Signature, Value};

use std::cell::{Ref, RefCell};
use std::collections::btree_map::{BTreeMap, Entry};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Map<T> = BTreeMap<String, T>;

pub struct Argument {
    name: String,
    signature: String,
}

impl Argument {
    pub fn new(name: &str, sig: &str) -> Self {
        // TODO: make a builder for the signature type.
        Argument {
            name: name.to_owned(),
            signature: sig.to_owned(),
        }
    }
}

pub struct Annotation {
    name: String,
    value: String,
}
type Annotations = Vec<Annotation>;

impl Annotation {
    pub fn new(name: &str, value: &str) -> Self {
        Annotation {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

pub struct ErrorMessage {
    name: String,
    message: String,
}

impl ErrorMessage {
    pub fn new(name: &str, message: &str) -> Self {
        ErrorMessage {
            name: name.to_owned(),
            message: message.to_owned(),
        }
    }

    fn to_message(self, msg: &Message) -> Message {
        msg.error_message(&self.name)
           .add_argument(&self.message)
    }
}

pub type MethodResult = Result<Vec<Value>, ErrorMessage>;
pub type MethodHandler = Rc<RefCell<FnMut(&mut Message) -> MethodResult>>;

pub struct Method {
    in_args: Vec<Argument>,
    out_args: Vec<Argument>,
    cb: MethodHandler,
    anns: Annotations,
}

impl Method {
    pub fn new<F>(cb: F) -> Self
        where F: FnMut(&mut Message) -> MethodResult + 'static {
        Method {
            in_args: vec![],
            out_args: vec![],
            cb: Rc::new(RefCell::new(cb)),
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: Argument) -> Self {
        self.in_args.push(arg);

        self
    }

    pub fn add_result(mut self, arg: Argument) -> Self {
        self.out_args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }
}

pub type PropertyGetResult = Result<Value, ErrorMessage>;
pub type PropertySetResult = Result<(), ErrorMessage>;

pub trait PropertyReadHandler {
    fn get(&self) -> PropertyGetResult;
}

pub trait PropertyWriteHandler {
    fn set(&self, &Value) -> PropertySetResult;
}

pub trait PropertyReadWriteHandler {
    fn get(&self) -> PropertyGetResult;
    fn set(&self, &Value) -> PropertySetResult;
}

enum PropertyAccess {
    RO(Box<PropertyReadHandler>),
    RW(Box<PropertyReadWriteHandler>),
    WO(Box<PropertyWriteHandler>),
}

pub struct Property {
    signature: Signature,
    access: PropertyAccess,
    anns: Annotations,
}

impl Property {
    fn new(sig: Signature, access: PropertyAccess) -> Self {
        Property {
            signature: sig,
            access: access,
            anns: vec![],
        }
    }

    pub fn new_ro(sig: Signature, access: Box<PropertyReadHandler>) -> Self {
        Property::new(sig, PropertyAccess::RO(access))
    }

    pub fn new_rw(sig: Signature, access: Box<PropertyReadWriteHandler>) -> Self {
        Property::new(sig, PropertyAccess::RW(access))
    }

    pub fn new_wo(sig: Signature, access: Box<PropertyWriteHandler>) -> Self {
        Property::new(sig, PropertyAccess::WO(access))
    }

    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }
}

pub struct Signal {
    args: Vec<Argument>,
    anns: Annotations,
}

impl Signal {
    pub fn new() -> Self {
        Signal {
            args: vec![],
            anns: vec![],
        }
    }

    pub fn add_argument(mut self, arg: Argument) -> Self {
        self.args.push(arg);

        self
    }

    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }
}

pub struct Interface {
    methods: Map<Method>,
    properties: Map<Property>,
    signals: Map<Signal>,
    anns: Annotations,
}

impl Interface {
    pub fn new() -> Self {
        Interface {
            methods: Map::new(),
            properties: Map::new(),
            signals: Map::new(),
            anns: vec![],
        }
    }

    pub fn add_method(mut self, name: &str, method: Method) -> Self {
        self.methods.insert(name.to_owned(), method);

        self
    }

    pub fn add_property(mut self, name: &str, property: Property) -> Self {
        self.properties.insert(name.to_owned(), property);

        self
    }

    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }

    pub fn add_signal(mut self, name: &str, signal: Signal) -> Self {
        self.signals.insert(name.to_owned(), signal);

        self
    }

    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }

    fn _require_property(&self, name: &str) -> Result<&Property, ErrorMessage> {
        self.properties.get(name).ok_or(
            ErrorMessage::new("org.freedesktop.DBus.Error.UnknownProperty",
                              &format!("unknown property: {}", name)))
    }

    pub fn get_property_value(&self, name: &str) -> MethodResult {
        self._require_property(name).and_then(|prop| {
            match prop.access {
                // TODO: Verify that the signature matches the return.
                PropertyAccess::RO(ref ro) => ro.get().map(|v| vec![v]),
                PropertyAccess::RW(ref rw) => rw.get().map(|v| vec![v]),
                PropertyAccess::WO(_) =>
                    Err(ErrorMessage {
                        name: "org.freedesktop.DBus.Error.Failed".to_owned(),
                        message: format!("property is write-only: {}", name),
                    }),
            }
        })
    }

    pub fn set_property_value(&self, name: &str, value: &Value) -> MethodResult {
        self._require_property(name).and_then(|prop| {
            match prop.access {
                PropertyAccess::WO(ref wo) => wo.set(value).map(|_| vec![]),
                PropertyAccess::RW(ref rw) => rw.set(value).map(|_| vec![]),
                PropertyAccess::RO(_) =>
                    Err(ErrorMessage::new("org.freedesktop.DBus.Error.Failed",
                                          &format!("property is read-only: {}", name))),
            }
        })
    }

    pub fn get_property_map(&self) -> Dictionary {
        Dictionary::new(self.properties.iter().map(|(k, v)| {
            match v.access {
                // TODO: Message that failures occurred?
                // TODO: Verify that the signature matches the return.
                PropertyAccess::RO(ref ro) => ro.get().ok(),
                PropertyAccess::RW(ref rw) => rw.get().ok(),
                PropertyAccess::WO(_)      => None,
            }.map(|v| {
                (BasicValue::String(k.clone()), v)
            })
        }).filter_map(|a| a).collect::<HashMap<BasicValue, Value>>())
    }
}

type InterfaceMap = Rc<RefCell<Map<Interface>>>;
type InterfaceMapRef = Weak<RefCell<Map<Interface>>>;
pub type ChildrenList = Rc<RefCell<Vec<String>>>;
type ChildrenListRef = Weak<RefCell<Vec<String>>>;

fn require_interface<'a>(map: &'a Ref<'a, Map<Interface>>, name: &str) -> Result<&'a Interface, ErrorMessage> {
    map.get(name).ok_or(
        ErrorMessage {
            name: "org.freedesktop.DBus.Error.UnknownInterface".to_owned(),
            message: format!("unknown interface: {}", name),
        })
}

pub struct Interfaces {
    map: InterfaceMap,
    finalized: bool,
}

struct PeerInterface;

impl PeerInterface {
    fn ping() -> MethodResult {
        Ok(vec![])
    }

    fn get_machine_id() -> MethodResult {
        let mid = format!("{}", MachineId::get());
        Ok(vec![Value::BasicValue(BasicValue::String(mid))])
    }

    pub fn new() -> Interface {
        Interface::new()
            .add_method("Ping", Method::new(|_| Self::ping()))
            .add_method("GetMachineId", Method::new(|_| Self::get_machine_id())
                .add_result(Argument::new("machine_uuid", "s")))
    }
}

struct PropertyInterface;

impl PropertyInterface {
    fn get_property(map: InterfaceMapRef, m: &mut Message) -> MethodResult {
        let values = try!(Arguments::new(m));
        let iface = try!(values.extract_string(0));
        let property = try!(values.extract_string(1));

        let smap = map.upgrade().expect("get_property: interface map no longer exists?");
        let smap_ref = &smap.borrow();

        require_interface(smap_ref, iface).and_then(|iface| {
            iface.get_property_value(property)
        })
    }

    fn set_property(map: InterfaceMapRef, m: &mut Message) -> MethodResult {
        let values = try!(Arguments::new(m));
        let iface = try!(values.extract_string(0));
        let property = try!(values.extract_string(1));
        let value = try!(values.extract(2));

        let smap = map.upgrade().expect("get_property: interface map no longer exists?");
        let smap_ref = &smap.borrow();

        require_interface(smap_ref, iface).and_then(|iface| {
            iface.set_property_value(property, value)
        })
    }

    fn get_all_properties(map: InterfaceMapRef, m: &mut Message) -> MethodResult {
        let values = try!(Arguments::new(m));
        let iface = try!(values.extract_string(0));

        let smap = map.upgrade().expect("get_property: interface map no longer exists?");
        let smap_ref = &smap.borrow();

        require_interface(smap_ref, iface).map(|iface| {
            vec![Value::Dictionary(iface.get_property_map())]
        })
    }

    pub fn new(map: InterfaceMapRef) -> Interface {
        let get_map = map.clone();
        let set_map = map.clone();
        let get_all_map = map.clone();

        Interface::new()
            .add_method("Get", Method::new(move |m| Self::get_property(get_map.clone(), m))
                .add_argument(Argument::new("interface_name", "s"))
                .add_argument(Argument::new("property_name", "s"))
                .add_result(Argument::new("value", "v")))
            .add_method("Set", Method::new(move |m| Self::set_property(set_map.clone(), m))
                .add_argument(Argument::new("interface_name", "s"))
                .add_argument(Argument::new("property_name", "s"))
                .add_result(Argument::new("value", "v")))
            .add_method("GetAll", Method::new(move |m| Self::get_all_properties(get_all_map.clone(), m))
                .add_argument(Argument::new("interface_name", "s"))
                .add_result(Argument::new("props", "{sv}")))
    }
}

struct IntrospectableInterface;

impl IntrospectableInterface {
    fn introspect(map: InterfaceMapRef, children: ChildrenListRef, _: &mut Message) -> MethodResult {
        let smap = map.upgrade().unwrap();
        let schildren = children.upgrade().unwrap();

        let xml = format!(concat!(
            r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"\n"#,
            r#" "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">\n"#,
            r#"<!-- rust-bus {} -->"#,
            r#"<node>\n"#,
            r#"{}"#, // interface
            r#"{}"#, // children
            r#"</node>\n"#),
            env!("CARGO_PKG_VERSION"),
            Self::_to_string_map(&*smap.borrow(), |k, v| Self::_introspect_interface(" ", k, v)),
            schildren.borrow().iter().fold("".to_owned(), |p, name| {
                format!(r#"{} <node name="{}" />"#, p, name)
            }));
        Ok(vec![Value::BasicValue(BasicValue::String(xml))])
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

    fn _introspect_annotation(indent: &str, ann: &Annotation) -> String {
        format!(r#"{}<annotation name="{}" value="{}" />\n"#,
            indent,
            ann.name,
            ann.value)
    }

    fn _introspect_arg(indent: &str, direction: &str, arg: &Argument) -> String {
        format!(r#"{}<arg name="{}" type="{}" direction="{}" />\n"#,
            indent,
            arg.name,
            arg.signature,
            direction)
    }

    fn _introspect_property(indent: &str, name: &String, prop: &Property) -> String {
        let new_indent = format!("{} ", indent);
        let access =
            match prop.access {
                PropertyAccess::RO(_) => "read",
                PropertyAccess::RW(_) => "readwrite",
                PropertyAccess::WO(_) => "write",
            };
        let sig = match prop.signature { Signature(ref s) => s };
        format!(r#"{}<property name="" type="{}" access="{}">\n{}{}</property>\n"#,
            name,
            sig,
            access,
            Self::_to_string_list(&prop.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_method(indent: &str, name: &String, method: &Method) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<method name="">\n{}{}{}{}</method>\n"#,
            name,
            Self::_to_string_list(&method.in_args, |t| Self::_introspect_arg(&new_indent, "in", t)),
            Self::_to_string_list(&method.out_args, |t| Self::_introspect_arg(&new_indent, "out", t)),
            Self::_to_string_list(&method.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_signal(indent: &str, name: &String, signal: &Signal) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<signal name="">\n{}{}{}</signal>\n"#,
            name,
            Self::_to_string_list(&signal.args, |t| Self::_introspect_arg(&new_indent, "out", t)),
            Self::_to_string_list(&signal.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    fn _introspect_interface(indent: &str, name: &String, iface: &Interface) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<interface name="{}">\n{}{}{}{}{}</interface>\n"#,
            indent,
            name,
            Self::_to_string_map(&iface.properties, |k, v| Self::_introspect_property(&new_indent, k, v)),
            Self::_to_string_map(&iface.methods, |k, v| Self::_introspect_method(&new_indent, k, v)),
            Self::_to_string_map(&iface.signals, |k, v| Self::_introspect_signal(&new_indent, k, v)),
            Self::_to_string_list(&iface.anns, |t| Self::_introspect_annotation(&new_indent, t)),
            indent)
    }

    pub fn new(map: InterfaceMapRef, children: ChildrenListRef) -> Interface {
        Interface::new()
            .add_method("Introspect", Method::new(move |m| Self::introspect(map.clone(), children.clone(), m))
                .add_result(Argument::new("xml_data", "s")))
    }
}

struct CallHeaders {
    interface: String,
    method: String,
}

impl CallHeaders {
    pub fn new(msg: &Message) -> Option<Self> {
        msg.interface().and_then(|interface| {
            msg.member().map(|method| {
                CallHeaders {
                    interface: interface,
                    method: method,
                }
            })
        })
    }

}

impl Interfaces {
    pub fn new() -> Self {
        Interfaces {
            map: Rc::new(RefCell::new(Map::new())),
            finalized: false,
        }
    }

    // Marked as mut for intent; Rc<> doesn't require it though.
    #[allow(unused_mut)]
    pub fn add_interface(mut self, name: &str, iface: Interface) -> Result<Self, Error> {
        if self.finalized {
            return Err(Error::InterfacesFinalized(name.to_owned()));
        }

        {
            let mut map = self.map.borrow_mut();

            match map.entry(name.to_owned()) {
                Entry::Vacant(v)    => {
                    v.insert(iface);

                    Ok(())
                },
                Entry::Occupied(_)  => Err(Error::InterfaceAlreadyRegistered(name.to_owned())),
            }
        }.map(|_| self)
    }

    pub fn finalize(mut self, children: ChildrenList) -> Result<Self, Error> {
        self = try!(Ok(self)
                .and_then(|this| {
                    this.add_interface("org.freedesktop.DBus.Peer",
                                       PeerInterface::new())
                }).and_then(|this| {
                    let map_ref = Rc::downgrade(&this.map);
                    this.add_interface("org.freedesktop.DBus.Properties",
                                       PropertyInterface::new(map_ref))
                }).and_then(|this| {
                    let map_ref = Rc::downgrade(&this.map);
                    this.add_interface("org.freedesktop.DBus.Introspectable",
                                       IntrospectableInterface::new(map_ref,
                                                                    Rc::downgrade(&children)))
                }));

        self.finalized = true;
        Ok(self)
    }

    fn _signature(args: &Vec<Argument>) -> String {
        args.iter()
            .map(|arg| arg.signature.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    fn _msg_signature(msg: &Message) -> String {
        msg.values().unwrap()
           .map_or_else(|| "".to_owned(),
                        |vs| vs.iter()
                               .map(|v| v.get_signature().to_owned())
                               .collect::<Vec<_>>()
                               .join(""))
    }

    fn _check_signature(args: &Vec<Argument>, msg: &Message) -> bool {
        let expect_sig = Self::_signature(args);
        let actual_sig = Self::_msg_signature(msg);

        expect_sig == actual_sig
    }

    pub fn handle(&self, conn: &Connection, msg: &mut Message) -> Option<Result<(), ()>> {
        CallHeaders::new(msg).and_then(|hdrs| {
            let iface_name = hdrs.interface;
            let method_name = hdrs.method;
            let map_ref = &self.map.borrow();
            let method = map_ref.get(&iface_name)
                                .and_then(|iface| iface.methods.get(&method_name));

            method.map(|method| {
                let res = if Self::_check_signature(&method.in_args, msg) {
                    let mut cb = method.cb.borrow_mut();

                    match cb.deref_mut()(msg) {
                        Ok(vals) => {
                            vals.iter().fold(msg.return_message(), |msg, val| {
                                msg.add_argument(val)
                            })
                        },
                        Err(err) => err.to_message(msg),
                    }
                } else {
                    Arguments::invalid_arguments()
                        .to_message(msg)
                };

                match res.message_type() {
                    MessageType::Error          => (),
                    MessageType::MethodReturn   => {
                        let expect = Self::_signature(&method.out_args);
                        let actual = Self::_msg_signature(&res);

                        if expect != actual {
                            panic!("invalid return signature for: \
                                    path: '{:?}' interface: '{}' method: '{}' \
                                    expected: '{}' actual: '{}'",
                                   msg.path(), iface_name, method_name,
                                   expect, actual)
                        };
                    },
                    _                           => {
                        panic!("invalid return value for: \
                                path: '{:?}' interface: '{}' method: '{}'",
                               msg.path(), iface_name, method_name)
                    },
                };

                conn.send(res)
                    .map(|_| ())
                    .map_err(|_| ())
            })
        })
    }
}

#[test]
fn empty_interface() {
    use super::connection::RequestNameFlags;
    use super::connection::RequestNameReply;

    let ifaces = Interfaces::new();
    let children = Rc::new(RefCell::new(vec![]));

    assert_eq!(ifaces.finalized, false);

    let ifaces = ifaces.finalize(children.clone()).unwrap();

    assert_eq!(ifaces.finalized, true);

    {
        let map = ifaces.map.borrow();
        assert_eq!(map.len(), 3);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Peer"), true);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Properties"), true);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Introspectable"), true);
    }

    let conn = Connection::session_new().unwrap();
    let name = "net.benboeckel.test.rustbus";

    assert_eq!(conn.request_name(name, RequestNameFlags::empty()).unwrap(), RequestNameReply::PrimaryOwner);

    let mut msg = Message::new_method_call(name, "/", "org.freedesktop.DBus.Introspectable", "Introspect");

    ifaces.handle(&conn, &mut msg);
}
