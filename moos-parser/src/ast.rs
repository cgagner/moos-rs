use std::{
    collections::HashMap,
    fmt::{Debug, Error, Formatter},
    ops::Index,
};

macro_rules! add_as {
    ($name:ident, $t:ty) => {
        fn $name(&self) -> Option<$t> {
            match *self {
                Self::Boolean(b) => Some(b as $t),
                Self::Integer(i) => Some(i as $t),
                Self::Float(f) if f.is_finite() => Some(f as $t),
                Self::String(ref s) => {
                    if let Ok(i) = str::parse::<$t>(s) {
                        Some(i)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    };
}
macro_rules! add_as_unsigned {
    ($name:ident, $t:ty) => {
        fn $name(&self) -> Option<$t> {
            match *self {
                Self::Boolean(b) => Some(b as $t),
                Self::Integer(i) if i >= 0 => Some(i as $t),
                Self::Float(f) if f.is_finite() && f >= 0.0 => Some(f as $t),
                Self::String(ref s) => {
                    if let Ok(i) = str::parse::<$t>(s) {
                        Some(i)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    };
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Param {
    name: String,
    value: Value,
}

type Params = Vec<Param>;

/**
 * Parameter value after evaluating environment variables. The underlying data
 * can only be a `f64`, `i64`, `bool`, or `String`. The values can be convert
 * into smaller sized integers and unsigned integers.
 */
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Value {
    #[default]
    /// No value
    None,
    /// Floating point value
    Float(f64),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// String value
    String(String),
    /// A `Config` is a collection of name value pairs
    Config(Params),
}

#[inline]
fn fequals(lhs: f64, rhs: f64, eps: f64) -> bool {
    if lhs.is_finite() && rhs.is_finite() {
        (lhs - rhs).abs() < eps
    } else if lhs.is_nan() && rhs.is_nan() {
        true
    } else if lhs.is_infinite() && rhs.is_infinite() {
        true
    } else {
        false
    }
}

impl Value {
    fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(b) => Some(b),
            Self::Integer(i) if i == 1 => Some(true),
            Self::Integer(i) if i == 0 => Some(false),
            Self::Float(f) if fequals(f, 0.0, 0.00001) => Some(false),
            Self::Float(f) if fequals(f, 1.0, 0.00001) => Some(true),
            Self::String(ref s) => match s.trim().to_uppercase().as_str() {
                "TRUE" | "YES" | "1" => Some(true),
                "FALSE" | "NO" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Boolean(b) => {
                if b {
                    Some(1.0)
                } else {
                    Some(0.0)
                }
            }
            Self::Integer(i) if i == 1 => Some(i as f64),
            Self::Float(f) => Some(f),
            Self::String(ref s) => {
                if let Ok(f) = str::parse::<f64>(s) {
                    Some(f)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    add_as!(as_i8, i8);
    add_as!(as_i16, i16);
    add_as!(as_i32, i32);
    add_as!(as_i64, i64);
    add_as_unsigned!(as_u8, u8);
    add_as_unsigned!(as_u16, u16);
    add_as_unsigned!(as_u32, u32);
    add_as_unsigned!(as_u64, u64);
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

static NO_VALUE: Value = Value::None;

impl Index<&str> for Value {
    type Output = Self;

    fn index(&self, index: &str) -> &Self::Output {
        match *self {
            Self::Config(ref params) => {
                for param in params {
                    if param.name.eq_ignore_ascii_case(index) {
                        return &param.value;
                    }
                }
                return &NO_VALUE;
            }
            _ => &NO_VALUE,
        }
    }
}

/**
 * Mission files are line-based key-value pairs. Additionally, missions
 * contain blocks of key-value pairs for processes (a.k.a ProcessConfig).
 * Comments and invalid lines are thrown out.
 */
#[derive(Debug, Clone)]
pub enum MissionLine {
    Param(Param),
    ProcessConfig {
        process_name: String,
        params: Vec<Param>,
    },
}

pub trait MissionContext {
    fn insert_param(&mut self, name: &str, value: Value);

    fn insert_process_param(&mut self, process: &str, name: &str, value: Value);

    fn get_param(&self, name: &str) -> Option<Value>;

    fn get_process_param(&self, process: &str, name: &str) -> Option<Value>;

    // TODO: Need to be able it iterator over process params.
    //fn get_process_params(&self, process: &str) -> Vec<Param>;
}

#[derive(Debug, Default, Clone)]
struct PreservedMissionContext {
    lines: Vec<MissionLine>,
}

impl MissionContext for PreservedMissionContext {
    fn insert_param(&mut self, name: &str, value: Value) {
        self.lines.push(MissionLine::Param(Param {
            name: name.to_owned(),
            value,
        }));
    }

    fn insert_process_param(&mut self, process: &str, name: &str, value: Value) {
        for line in &mut self.lines {
            match line {
                MissionLine::ProcessConfig {
                    process_name,
                    params,
                } if process_name.eq_ignore_ascii_case(process) => {
                    params.push(Param {
                        name: name.to_owned(),
                        value,
                    });
                    return;
                }
                _ => {}
            }
        }
    }

    fn get_param(&self, name: &str) -> Option<Value> {
        for line in &self.lines {
            match line {
                MissionLine::Param(param) if param.name.eq_ignore_ascii_case(name) => {
                    return Some(param.value.to_owned());
                }
                _ => {}
            }
        }
        None
    }

    fn get_process_param(&self, process: &str, name: &str) -> Option<Value> {
        for line in &self.lines {
            match line {
                MissionLine::ProcessConfig {
                    process_name,
                    params,
                } if process_name.eq_ignore_ascii_case(process) => {
                    for param in params {
                        if param.name.eq_ignore_ascii_case(name) {
                            return Some(param.value.to_owned());
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// TODO: Should the Unordered Mission Context check the case?

#[derive(Debug, Default, Clone)]
struct UnorderedMissionContext {
    params: HashMap<String, Value>,
    process_configs: HashMap<String, HashMap<String, Value>>,
}

impl MissionContext for UnorderedMissionContext {
    fn insert_param(&mut self, name: &str, value: Value) {
        self.params.insert(name.to_owned(), value);
    }

    fn insert_process_param(&mut self, process: &str, name: &str, value: Value) {
        self.process_configs
            .entry(process.to_owned())
            .or_insert(HashMap::new())
            .insert(name.to_owned(), value);
    }

    fn get_param(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.params.get(name) {
            Some(value.to_owned())
        } else {
            None
        }
    }

    fn get_process_param(&self, process: &str, name: &str) -> Option<Value> {
        if let Some(params) = self.process_configs.get(process) {
            if let Some(value) = params.get(name) {
                return Some(value.clone());
            }
        }
        None
    }
}

pub struct Mission<T: MissionContext> {
    context: T,
}

impl<T: MissionContext> Mission<T> {
    #[inline]
    fn insert_param(&mut self, name: &str, value: Value) {
        self.context.insert_param(name, value)
    }

    #[inline]
    fn insert_process_param(&mut self, process: &str, name: &str, value: Value) {
        self.context.insert_process_param(process, name, value)
    }

    #[inline]
    fn get_param(&self, name: &str) -> Option<Value> {
        self.context.get_param(name)
    }

    #[inline]
    fn get_param_or_default(&self, name: &str, default_value: Value) -> Value {
        if let Some(value) = self.get_param(name) {
            value
        } else {
            default_value
        }
    }

    #[inline]
    fn get_process_param(&self, process: &str, name: &str) -> Option<Value> {
        self.context.get_process_param(process, name)
    }

    #[inline]
    fn get_process_param_or_default(
        &self,
        process: &str,
        name: &str,
        default_value: Value,
    ) -> Value {
        if let Some(value) = self.get_process_param(process, name) {
            value
        } else {
            default_value
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::{Param, Params, Value};

    #[test]
    fn test_value_index() {
        let mut params = vec![];
        params.push(Param {
            name: "test".to_owned(),
            value: Value::Float(12.32),
        });

        params.push(Param {
            name: "test_bool_str".to_owned(),
            value: Value::String("false".to_owned()),
        });

        let value: Value = Value::from(10.0);

        let key = String::from("test");

        let config = Value::Config(params);

        assert_eq!(config["test"], Value::Float(12.32));
        assert_eq!(config[&key], Value::Float(12.32));

        assert_eq!(config["test_bool_str"].as_bool(), Some(false));
        assert_eq!(config["not_found"], Value::None);
    }
}
