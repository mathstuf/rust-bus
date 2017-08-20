// Distributed under the OSI-approved BSD 3-Clause License.
// See accompanying LICENSE file for details.

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
use std::rc::{Rc, Weak};

type Map<T> = BTreeMap<String, T>;

/// An argument to a method or signal.
pub struct Argument {
    name: String,
    signature: String,
}

impl Argument {
    /// Create a new argument.
    ///
    /// The signature string specification is documented in the [D-Bus
    /// specification](https://dbus.freedesktop.org/doc/dbus-specification.html#basic-types).
    pub fn new(name: &str, sig: &str) -> Self {
        // TODO: make a builder for the signature type.
        Argument {
            name: name.to_owned(),
            signature: sig.to_owned(),
        }
    }
}

/// Metadata to attach to methods, signals, and properties.
///
/// Annotations are used to convey information such as whether a property is observable,
/// deprecated, the method does not reply, or whether the property value may be cached in client
/// code.
pub struct Annotation {
    name: String,
    value: String,
}
type Annotations = Vec<Annotation>;

impl Annotation {
    /// Create a new annotation.
    ///
    /// For some well-known annotations, see the [D-Bus
    /// specification](https://dbus.freedesktop.org/doc/dbus-specification.html#introspection-format).
    pub fn new(name: &str, value: &str) -> Self {
        Annotation {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

/// An error message from a method call.
pub struct ErrorMessage {
    name: String,
    message: String,
}

impl ErrorMessage {
    /// Create a new error message.
    ///
    /// Error message names usually contain `.Error.`.
    pub fn new(name: &str, message: &str) -> Self {
        ErrorMessage {
            name: name.to_owned(),
            message: message.to_owned(),
        }
    }

    fn into_message(self, msg: &Message) -> Message {
        msg.error_message(&self.name)
            .add_argument(&self.message)
    }
}

/// The result of a method call.
pub type MethodResult = Result<Vec<Value>, ErrorMessage>;
/// A holder for method closures.
pub type MethodHandler = Box<RefCell<FnMut(&mut Message) -> MethodResult>>;

/// A representation of a method call.
pub struct Method {
    in_args: Vec<Argument>,
    out_args: Vec<Argument>,
    cb: MethodHandler,
    anns: Annotations,
}

impl Method {
    /// Create a new `Method` with the given function.
    pub fn new<F>(cb: F) -> Self
        where F: FnMut(&mut Message) -> MethodResult + 'static
    {
        Method {
            in_args: vec![],
            out_args: vec![],
            cb: Box::new(RefCell::new(cb)),
            anns: vec![],
        }
    }

    /// Add an input argument to the method.
    pub fn add_argument(mut self, arg: Argument) -> Self {
        self.in_args.push(arg);

        self
    }

    /// Add an output to the method.
    pub fn add_result(mut self, arg: Argument) -> Self {
        self.out_args.push(arg);

        self
    }

    /// Add an annotation to the method.
    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }
}

/// The result of a property query.
pub type PropertyGetResult = Result<Value, ErrorMessage>;
/// The result of a property setting.
pub type PropertySetResult = Result<(), ErrorMessage>;

/// A trait for read-only properties.
pub trait PropertyReadHandler {
    /// Get the value of the property.
    fn get(&self) -> PropertyGetResult;
}

/// A trait for write-only properties.
pub trait PropertyWriteHandler {
    /// Set the value of the property.
    fn set(&self, &Value) -> PropertySetResult;
}

/// A trait for read-write properties.
pub trait PropertyReadWriteHandler {
    /// Get the value of the property.
    fn get(&self) -> PropertyGetResult;
    /// Set the value of the property.
    fn set(&self, &Value) -> PropertySetResult;
}

enum PropertyAccess {
    RO(Box<PropertyReadHandler>),
    RW(Box<PropertyReadWriteHandler>),
    WO(Box<PropertyWriteHandler>),
}

/// A property which is exposed over the bus.
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

    /// Create a new read-only property.
    pub fn new_ro(sig: Signature, access: Box<PropertyReadHandler>) -> Self {
        Property::new(sig, PropertyAccess::RO(access))
    }

    /// Create a new read-write property.
    pub fn new_rw(sig: Signature, access: Box<PropertyReadWriteHandler>) -> Self {
        Property::new(sig, PropertyAccess::RW(access))
    }

    /// Create a new write-only property.
    pub fn new_wo(sig: Signature, access: Box<PropertyWriteHandler>) -> Self {
        Property::new(sig, PropertyAccess::WO(access))
    }

    /// Add an annotation to the property.
    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }

    fn _check_signature(&self, value: &Value) -> bool {
        self.signature.0 == value.get_signature()
    }
}

#[derive(Default)]
/// A signal which may be emitted by the server.
pub struct Signal {
    args: Vec<Argument>,
    anns: Annotations,
}

impl Signal {
    /// Create a new signal.
    pub fn new() -> Self {
        Signal {
            args: vec![],
            anns: vec![],
        }
    }

    /// Add an argument to the signal.
    pub fn add_argument(mut self, arg: Argument) -> Self {
        self.args.push(arg);

        self
    }

    /// Add an annotation to the signal.
    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }
}

#[derive(Default)]
/// A representation of an interface.
pub struct Interface {
    methods: Map<Method>,
    properties: Map<Property>,
    signals: Map<Signal>,
    anns: Annotations,
}

impl Interface {
    /// Create a new interface.
    pub fn new() -> Self {
        Interface {
            methods: Map::new(),
            properties: Map::new(),
            signals: Map::new(),
            anns: vec![],
        }
    }

    /// Add a method to the interface.
    pub fn add_method(mut self, name: &str, method: Method) -> Self {
        self.methods.insert(name.to_owned(), method);

        self
    }

    /// Add a property to the interface.
    pub fn add_property(mut self, name: &str, property: Property) -> Self {
        self.properties.insert(name.to_owned(), property);

        self
    }

    /// Get a property from the interface.
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }

    /// Add a signal to the interface.
    pub fn add_signal(mut self, name: &str, signal: Signal) -> Self {
        self.signals.insert(name.to_owned(), signal);

        self
    }

    /// Add an annotation to the interface.
    pub fn annotate(mut self, ann: Annotation) -> Self {
        self.anns.push(ann);

        self
    }

    fn _require_property(&self, name: &str) -> Result<&Property, ErrorMessage> {
        self.properties.get(name).ok_or_else(|| {
            ErrorMessage::new("org.freedesktop.DBus.Error.UnknownProperty",
                              &format!("unknown property: {}", name))
        })
    }

    /// Get the value of a property.
    pub fn get_property_value(&self, name: &str) -> MethodResult {
        self._require_property(name).and_then(|prop| {
            let res = match prop.access {
                PropertyAccess::RO(ref ro) => ro.get(),
                PropertyAccess::RW(ref rw) => rw.get(),
                PropertyAccess::WO(_) => {
                    Err(ErrorMessage {
                        name: "org.freedesktop.DBus.Error.Failed".to_owned(),
                        message: format!("property is write-only: {}", name),
                    })
                },
            };

            if let Ok(value) = res.as_ref() {
                if prop._check_signature(value) {
                    panic!("invalid property return type for: \
                            property: '{}' expected: '{}' actual: '{}'",
                           name,
                           value.get_signature(),
                           prop.signature.0)
                }
            }

            res.map(|v| vec![v])
        })
    }

    /// Set a property value.
    pub fn set_property_value(&self, name: &str, value: &Value) -> MethodResult {
        self._require_property(name).and_then(|prop| {
            if prop._check_signature(value) {
                return Err(Arguments::invalid_arguments());
            }

            match prop.access {
                PropertyAccess::WO(ref wo) => wo.set(value).map(|_| vec![]),
                PropertyAccess::RW(ref rw) => rw.set(value).map(|_| vec![]),
                PropertyAccess::RO(_) => {
                    Err(ErrorMessage::new("org.freedesktop.DBus.Error.Failed",
                                          &format!("property is read-only: {}", name)))
                },
            }
        })
    }

    /// Get a map of all (readable) property values.
    pub fn get_property_map(&self) -> Dictionary {
        Dictionary::new(self.properties
            .iter()
            .map(|(k, v)| {
                match v.access {
                        PropertyAccess::RO(ref ro) => ro.get().ok(),
                        PropertyAccess::RW(ref rw) => rw.get().ok(),
                        PropertyAccess::WO(_) => None,
                    }
                    .map(|v| (BasicValue::String(k.clone()), v))
            })
            .filter_map(|a| a)
            .collect())
    }
}

type InterfaceMap = Rc<RefCell<Map<Interface>>>;
type InterfaceMapRef = Weak<RefCell<Map<Interface>>>;
/// A list of child objects for an object.
pub type ChildrenList = Rc<RefCell<Vec<String>>>;
type ChildrenListRef = Weak<RefCell<Vec<String>>>;

fn require_interface<'a>(map: &'a Ref<'a, Map<Interface>>, name: &str)
                         -> Result<&'a Interface, ErrorMessage> {
    map.get(name).ok_or(ErrorMessage {
        name: "org.freedesktop.DBus.Error.UnknownInterface".to_owned(),
        message: format!("unknown interface: {}", name),
    })
}

/// A builder for a set of interfaces that an object implements.
pub struct InterfacesBuilder {
    map: InterfaceMap,
}

/// A set of interfaces that an object implements.
pub struct Interfaces {
    map: InterfaceMap,
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
            .add_method("GetMachineId",
                        Method::new(|_| Self::get_machine_id())
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

        require_interface(smap_ref, iface).and_then(|iface| iface.get_property_value(property))
    }

    fn set_property(map: InterfaceMapRef, m: &mut Message) -> MethodResult {
        let values = try!(Arguments::new(m));
        let iface = try!(values.extract_string(0));
        let property = try!(values.extract_string(1));
        let value = try!(values.extract(2));

        let smap = map.upgrade().expect("get_property: interface map no longer exists?");
        let smap_ref = &smap.borrow();

        require_interface(smap_ref, iface)
            .and_then(|iface| iface.set_property_value(property, value))
    }

    fn get_all_properties(map: InterfaceMapRef, m: &mut Message) -> MethodResult {
        let values = try!(Arguments::new(m));
        let iface = try!(values.extract_string(0));

        let smap = map.upgrade().expect("get_property: interface map no longer exists?");
        let smap_ref = &smap.borrow();

        require_interface(smap_ref, iface)
            .map(|iface| vec![Value::Dictionary(iface.get_property_map())])
    }

    pub fn new(map: InterfaceMapRef) -> Interface {
        let get_map = map.clone();
        let set_map = map.clone();
        let get_all_map = map.clone();

        Interface::new()
            .add_method("Get",
                        Method::new(move |m| Self::get_property(get_map.clone(), m))
                            .add_argument(Argument::new("interface_name", "s"))
                            .add_argument(Argument::new("property_name", "s"))
                            .add_result(Argument::new("value", "v")))
            .add_method("Set",
                        Method::new(move |m| Self::set_property(set_map.clone(), m))
                            .add_argument(Argument::new("interface_name", "s"))
                            .add_argument(Argument::new("property_name", "s"))
                            .add_result(Argument::new("value", "v")))
            .add_method("GetAll",
                        Method::new(move |m| Self::get_all_properties(get_all_map.clone(), m))
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
                          Self::_to_string_map(&*smap.borrow(),
                                               |k, v| Self::_introspect_interface(" ", k, v)),
                          schildren.borrow().iter().fold("".to_owned(), |p, name| {
                              format!(r#"{} <node name="{}" />"#, p, name)
                          }));
        Ok(vec![Value::BasicValue(BasicValue::String(xml))])
    }

    fn _to_string_map<K, V, F>(map: &BTreeMap<K, V>, f: F) -> String
        where F: Fn(&K, &V) -> String
    {
        map.iter().fold("".to_owned(), |p, (k, v)| format!("{}{}", p, f(k, v)))
    }

    fn _to_string_list<T, F>(map: &[T], f: F) -> String
        where F: Fn(&T) -> String
    {
        map.iter().fold("".to_owned(), |p, t| format!("{}{}", p, f(t)))
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

    fn _introspect_property(indent: &str, name: &str, prop: &Property) -> String {
        let new_indent = format!("{} ", indent);
        let access = match prop.access {
            PropertyAccess::RO(_) => "read",
            PropertyAccess::RW(_) => "readwrite",
            PropertyAccess::WO(_) => "write",
        };
        let sig = match prop.signature {
            Signature(ref s) => s,
        };
        format!(r#"{}<property name="" type="{}" access="{}">\n{}{}</property>\n"#,
                name,
                sig,
                access,
                Self::_to_string_list(&prop.anns, |t| Self::_introspect_annotation(&new_indent, t)),
                indent)
    }

    fn _introspect_method(indent: &str, name: &str, method: &Method) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<method name="">\n{}{}{}{}</method>\n"#,
                name,
                Self::_to_string_list(&method.in_args,
                                      |t| Self::_introspect_arg(&new_indent, "in", t)),
                Self::_to_string_list(&method.out_args,
                                      |t| Self::_introspect_arg(&new_indent, "out", t)),
                Self::_to_string_list(&method.anns,
                                      |t| Self::_introspect_annotation(&new_indent, t)),
                indent)
    }

    fn _introspect_signal(indent: &str, name: &str, signal: &Signal) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<signal name="">\n{}{}{}</signal>\n"#,
                name,
                Self::_to_string_list(&signal.args,
                                      |t| Self::_introspect_arg(&new_indent, "out", t)),
                Self::_to_string_list(&signal.anns,
                                      |t| Self::_introspect_annotation(&new_indent, t)),
                indent)
    }

    fn _introspect_interface(indent: &str, name: &str, iface: &Interface) -> String {
        let new_indent = format!("{} ", indent);
        format!(r#"{}<interface name="{}">\n{}{}{}{}{}</interface>\n"#,
                indent,
                name,
                Self::_to_string_map(&iface.properties,
                                     |k, v| Self::_introspect_property(&new_indent, k, v)),
                Self::_to_string_map(&iface.methods,
                                     |k, v| Self::_introspect_method(&new_indent, k, v)),
                Self::_to_string_map(&iface.signals,
                                     |k, v| Self::_introspect_signal(&new_indent, k, v)),
                Self::_to_string_list(&iface.anns,
                                      |t| Self::_introspect_annotation(&new_indent, t)),
                indent)
    }

    pub fn new(map: InterfaceMapRef, children: ChildrenListRef) -> Interface {
        Interface::new().add_method("Introspect",
                                    Method::new(move |m| {
                                            Self::introspect(map.clone(), children.clone(), m)
                                        })
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

impl InterfacesBuilder {
    // Marked as mut for intent; Rc<> doesn't require it though.
    #[allow(unused_mut)]
    /// Add an interface to the set.
    pub fn add_interface(mut self, name: &str, iface: Interface) -> Result<Self, Error> {
        {
                let mut map = self.map.borrow_mut();

                match map.entry(name.to_owned()) {
                    Entry::Vacant(v) => {
                        v.insert(iface);

                        Ok(())
                    },
                    Entry::Occupied(_) => Err(Error::InterfaceAlreadyRegistered(name.to_owned())),
                }
            }
            .map(|_| self)
    }

    /// Finalize the interface set.
    ///
    /// Once this is called, the interfaces may be used fully. Calling this adds the
    /// `org.freedesktop.DBus.Peer`, `org.freedesktop.DBus.Properties`, and
    /// `org.freedesktop.DBus.Introspectable` standard interfaces to the object.
    ///
    /// Once this is called, further interfaces may not be added once this is called.
    pub fn finalize(mut self, children: &ChildrenList) -> Result<Interfaces, Error> {
        self = try!(Ok(self)
            .and_then(|this| {
                this.add_interface("org.freedesktop.DBus.Peer", PeerInterface::new())
            })
            .and_then(|this| {
                let map_ref = Rc::downgrade(&this.map);
                this.add_interface("org.freedesktop.DBus.Properties",
                                   PropertyInterface::new(map_ref))
            })
            .and_then(|this| {
                let map_ref = Rc::downgrade(&this.map);
                this.add_interface("org.freedesktop.DBus.Introspectable",
                                   IntrospectableInterface::new(map_ref, Rc::downgrade(children)))
            }));

        Ok(Interfaces {
            map: self.map,
        })
    }
}

impl Interfaces {
    /// Create a new, empty set of interfaces.
    pub fn new() -> InterfacesBuilder {
        InterfacesBuilder {
            map: Rc::new(RefCell::new(Map::new())),
        }
    }

    fn _signature(args: &[Argument]) -> String {
        args.iter()
            .map(|arg| arg.signature.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    fn _msg_signature(msg: &Message) -> String {
        msg.values()
            .unwrap()
            .map_or_else(|| "".to_owned(), |vs| {
                vs.iter()
                    .map(|v| v.get_signature().to_owned())
                    .collect::<Vec<_>>()
                    .join("")
            })
    }

    fn _check_signature(args: &[Argument], msg: &Message) -> bool {
        let expect_sig = Self::_signature(args);
        let actual_sig = Self::_msg_signature(msg);

        expect_sig == actual_sig
    }

    /// Return a dictionary of interfaces and properties on the interface.
    ///
    /// This is meant to be used by an ObjectManager interface.
    pub fn get_interfaces_and_properties(&self) -> Dictionary {
        Dictionary::new(self.map
            .borrow()
            .iter()
            .map(|(k, v)| (BasicValue::String(k.clone()), Value::Dictionary(v.get_property_map())))
            .collect())
    }

    /// Parse a `Message` and call the appropriate method (if applicable).
    ///
    /// Returns `None` if the method doesn't match, otherwise a a `Result` indicating whether the
    /// method call succeeded or not.
    ///
    /// # Panics
    ///
    /// If the method returns values which do not match its signature, a panic will occur since
    /// this is a bug in the implementation.
    pub fn handle(&self, conn: &Connection, msg: &mut Message) -> Option<Result<(), ()>> {
        CallHeaders::new(msg).map(|hdrs| {
            let iface_name = hdrs.interface;
            let method_name = hdrs.method;
            let map_ref = &self.map.borrow();
            let opt_iface = map_ref.get(&iface_name);
            let opt_method = opt_iface.and_then(|iface| iface.methods.get(&method_name));

            let res = if let Some(method) = opt_method {
                let res = if Self::_check_signature(&method.in_args, msg) {
                    let mut cb = method.cb.borrow_mut();

                    match cb.deref_mut()(msg) {
                        Ok(vals) => {
                            vals.iter().fold(msg.return_message(), |msg, val| msg.add_argument(val))
                        },
                        Err(err) => err.into_message(msg),
                    }
                } else {
                    Arguments::invalid_arguments().into_message(msg)
                };

                match res.message_type() {
                    MessageType::Error => (),
                    MessageType::MethodReturn => {
                        let expect = Self::_signature(&method.out_args);
                        let actual = Self::_msg_signature(&res);

                        if expect != actual {
                            panic!("invalid return signature for: \
                                    path: '{:?}' interface: '{}' method: '{}' \
                                    expected: '{}' actual: '{}'",
                                   msg.path(),
                                   iface_name,
                                   method_name,
                                   expect,
                                   actual)
                        };
                    },
                    _ => {
                        panic!("invalid return value for: \
                                path: '{:?}' interface: '{}' method: '{}'",
                               msg.path(),
                               iface_name,
                               method_name)
                    },
                };

                res
            } else if opt_iface.is_none() {
                msg.error_message("org.freedesktop.DBus.Error.UnknownMethod")
                    .add_argument(&format!("unknown interface: {}", iface_name))
            } else {
                msg.error_message("org.freedesktop.DBus.Error.UnknownMethod")
                    .add_argument(&format!("unknown method: {}", method_name))
            };

            conn.send(res)
                .map(|_| ())
                .map_err(|_| ())
        })
    }
}

#[test]
fn empty_interface() {
    use super::connection::RequestNameFlags;
    use super::connection::RequestNameReply;

    let ifaces = Interfaces::new();
    let children = Rc::new(RefCell::new(vec![]));

    let ifaces = ifaces.finalize(&children).unwrap();

    {
        let map = ifaces.map.borrow();
        assert_eq!(map.len(), 3);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Peer"), true);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Properties"), true);
        assert_eq!(map.contains_key("org.freedesktop.DBus.Introspectable"),
                   true);
    }

    let conn = Connection::session_new().unwrap();
    let name = "net.benboeckel.test.rustbus";

    assert_eq!(conn.request_name(name, RequestNameFlags::empty()).unwrap(),
               RequestNameReply::PrimaryOwner);

    let mut msg = Message::new_method_call(name,
                                           "/",
                                           "org.freedesktop.DBus.Introspectable",
                                           "Introspect");

    ifaces.handle(&conn, &mut msg);
}
