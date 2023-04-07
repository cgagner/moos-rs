use std::fmt::{Debug, Error, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}

impl Value {
    pub fn from_int(value: i64) {
        Value::Integer(value)
    }
}
