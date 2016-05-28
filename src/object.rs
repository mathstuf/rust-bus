use super::connection::Connection;
use super::error::Error;
use super::interface::{ChildrenList, Interfaces};
use super::message::Message;

pub struct Object {
    path: String,

    interfaces: Interfaces,
}

impl Object {
    pub fn new(path: &str, interfaces: Interfaces, children: ChildrenList) -> Result<Self, Error> {
        Ok(Object {
            path: path.to_owned(),
            interfaces: try!(interfaces.finalize(children)),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handle_message(&mut self, conn: &Connection, msg: &mut Message) -> Option<Result<(), ()>> {
        self.interfaces.handle(conn, msg)
    }
}
