# rust-bus

Module for exposing interfaces to a D-Bus manager.

The `bus` crate makes it easy to expose interfaces on the D-Bus. Objects are
created and added to servers which delegate messages across the objects. The
standard interfaces such as properties and introspection are provided
automatically.

Servers may also be created to listen for signals and handle them.

## TODO

Things are not yet complete, but here's a list of things that need to be done
(in rough order of importance):

  - Automatically request matches for servers which are created.
  - Implement the [`org.freedesktop.DBus.ObjectManager`][object-manager]
    interface.
  - Implement the
    [`org.freedesktop.DBus.Properties.PropertiesChanged`][properties] method.
  - Validate that object paths are valid.
  - Use a standard event loop (currently blocks).
  - Allow less common connection creation.
  - Create a tool to create bindings from XML (probably a separate repository).
  - Create a tool to create skeleton Rust code from XML (also a separate
    repository).
  - Make signature building easier.
  - Check that properties use the correct types which match their signatures.

[object-manager]: https://dbus.freedesktop.org/doc/dbus-specification.html#standard-interfaces-objectmanager
[properties]: https://dbus.freedesktop.org/doc/dbus-specification.html#standard-interfaces-properties
