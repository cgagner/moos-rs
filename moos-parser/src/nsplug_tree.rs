pub struct Define {
    name: String,
    value: String,
}

impl Define {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

struct Statements;

struct IfBlock {
    condition: Condition,
    statements: Statements,
}

enum Condition {
    Simple(String, String),
    OrCondition(Vec<(String, String)>),
    AndCondition(Vec<(String, String)>),
}

// enum PlugLine {
//   Define{name: String, value: String},
//   Include(file: String),
//   OtherLine(line: String),
// }
