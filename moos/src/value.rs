pub enum Value {
    /// Represents a value that hasn't been initialized
    Null,
    /// Represents a boolean
    Boolean(bool),
    /// Represents an integer
    Integer(i64),
    /// Represnts a 64-bit float
    Double(f64),
    DateTime(DateTime),
    Array(Array),
    Table(Table),
}

pub type Array = Vec<Value>;

pub type Table = Map<String, Value>;
