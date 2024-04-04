use crate::lexers::TokenRange;
use crate::vec_wrapper;

#[derive(Debug)]
pub enum Value<'input> {
    Boolean(bool, &'input str, TokenRange),
    Integer(i64, &'input str, TokenRange),
    Float(f64, &'input str, TokenRange),
    String(&'input str, TokenRange),
    Quote(Quote<'input>),
    Variable(Variable<'input>),
}

impl<'input> ToString for Value<'input> {
    fn to_string(&self) -> String {
        match self {
            Self::Boolean(_, value_str, _)
            | Self::Integer(_, value_str, _)
            | Self::Float(_, value_str, _)
            | Self::String(value_str, _) => (*value_str).to_string(),
            Self::Quote(quote) => quote.to_string(),
            Self::Variable(variable) => variable.to_string(),
        }
    }
}

impl<'input> From<Variable<'input>> for Value<'input> {
    fn from(value: Variable<'input>) -> Self {
        Self::Variable(value)
    }
}

impl<'input> TryFrom<Value<'input>> for Variable<'input> {
    type Error = ();

    fn try_from(value: Value<'input>) -> Result<Self, Self::Error> {
        match value {
            Value::Variable(variable) => Ok(variable),
            _ => Err(()),
        }
    }
}

// Declares a new struct Values that wraps a Vec<Value>
vec_wrapper!(Values, Value);

#[derive(Debug, Copy, Clone)]
pub enum Variable<'input> {
    Regular {
        text: &'input str,
        range: TokenRange,
    },
    Upper {
        text: &'input str,
        range: TokenRange,
    },
    Partial {
        text: &'input str,
        range: TokenRange,
    },
    PartialUpper {
        text: &'input str,
        range: TokenRange,
    },
}
impl<'input> ToString for Variable<'input> {
    fn to_string(&self) -> String {
        match self {
            Variable::Regular { text, range: _ } => format!("$({})", text),
            Variable::Upper { text, range: _ } => format!("%({})", text),
            Variable::Partial { text, range: _ } => format!("$({}", text),
            Variable::PartialUpper { text, range: _ } => format!("%({}", text),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VariableString<'input> {
    String(&'input str, TokenRange),
    Variable(Variable<'input>),
}

impl<'input> VariableString<'input> {
    #[inline]
    pub fn is_string(&self) -> bool {
        match *self {
            VariableString::String(_, _) => true,
            VariableString::Variable(_) => false,
        }
    }

    #[inline]
    pub fn is_variable(&self) -> bool {
        match *self {
            VariableString::String(_, _) => false,
            VariableString::Variable(_) => true,
        }
    }
}

impl<'input> ToString for VariableString<'input> {
    fn to_string(&self) -> String {
        match self {
            Self::String(value_str, _) => (*value_str).to_string(),
            // We won't evaluate plug variables as part of this parser.
            Self::Variable(variable) => variable.to_string(),
        }
    }
}

impl<'input> From<Variable<'input>> for VariableString<'input> {
    fn from(value: Variable<'input>) -> Self {
        Self::Variable(value)
    }
}

impl<'input> TryFrom<VariableString<'input>> for Variable<'input> {
    type Error = ();

    fn try_from(value: VariableString<'input>) -> Result<Self, Self::Error> {
        match value {
            VariableString::Variable(variable) => Ok(variable),
            _ => Err(()),
        }
    }
}

vec_wrapper!(VariableStrings, VariableString);

#[derive(Debug)]
pub struct Quote<'input> {
    pub content: VariableStrings<'input>,
    pub range: TokenRange,
}

impl<'input> ToString for Quote<'input> {
    fn to_string(&self) -> String {
        return format!("\"{}\"", self.content.eval());
    }
}

impl<'input> From<Quote<'input>> for Value<'input> {
    fn from(value: Quote<'input>) -> Self {
        Self::Quote(value)
    }
}

#[derive(Debug)]
pub enum IncludePath<'input> {
    VariableStrings(VariableStrings<'input>, TokenRange),
    Quote(Quote<'input>),
}

impl<'input> IncludePath<'input> {
    pub fn get_range(&self) -> &TokenRange {
        match self {
            IncludePath::VariableStrings(_, range) => range,
            IncludePath::Quote(quote) => &quote.range,
        }
    }
}

impl<'input> ToString for IncludePath<'input> {
    fn to_string(&self) -> String {
        match self {
            Self::VariableStrings(values, _) => values.to_string(),
            // We won't evaluate plug variables as part of this parser.
            Self::Quote(quote) => quote.to_string(),
        }
    }
}

impl<'input> From<Quote<'input>> for IncludePath<'input> {
    fn from(value: Quote<'input>) -> Self {
        Self::Quote(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncludeTag<'input> {
    pub tag: &'input str,
    /// Range of the Include tag. THis includes the start and ending brackets
    pub range: TokenRange,
}

impl<'input> IncludeTag<'input> {
    pub fn new(tag: &'input str, range: TokenRange) -> Self {
        Self { tag, range }
    }
}

impl<'input> ToString for IncludeTag<'input> {
    fn to_string(&self) -> String {
        format!("<{}>", self.tag)
    }
}

#[derive(Debug)]
pub enum MacroType<'input> {
    Define {
        definition: MacroDefinition<'input>,
        /// Range of the "#define"
        range: TokenRange,
    },
    Include {
        path: IncludePath<'input>,
        /// Optional include tag. Added in 2020.
        tag: Option<IncludeTag<'input>>,
        /// Range of the "#include"
        range: TokenRange,
    },
    IfDef {
        condition: MacroCondition<'input>,
        branch: IfDefBranch<'input>,
        body: Lines<'input>,
        /// Range of the "#ifdef"
        range: TokenRange,
    },
    IfNotDef {
        clauses: IfNotDefClauses<'input>,
        branch: IfNotDefBranch<'input>,
        body: Lines<'input>,
        /// Range of the "#ifndef"
        range: TokenRange,
    },
}

impl<'input> ToString for MacroType<'input> {
    fn to_string(&self) -> String {
        match self {
            MacroType::Define {
                definition,
                range: _,
            } => {
                format!("#define {}", definition.to_string())
            }
            MacroType::Include {
                path,
                tag,
                range: _,
            } => {
                if let Some(tag) = tag {
                    format!("#include {} {}", path.to_string(), tag.to_string())
                } else {
                    format!("#include {}", path.to_string())
                }
            }
            MacroType::IfDef {
                condition,
                branch: _,
                body: _,
                range: _,
            } => {
                // TODO: Need to recursively print the branch and lines
                format!("#ifdef {}", condition.to_string())
            }
            MacroType::IfNotDef {
                clauses,
                branch: _,
                body: _,
                range: _,
            } => {
                let rtn = "#ifndef ".to_string();
                clauses
                    .iter()
                    .fold(rtn, |acc, v| acc + " " + v.to_string().as_str())
            }
        }
    }
}

#[derive(Debug)]
pub struct MacroDefinition<'input> {
    pub name: VariableStrings<'input>,
    pub value: Values<'input>,
}

impl<'input> MacroDefinition<'input> {
    /// Create a new MacroDefinition
    pub fn new(name: VariableStrings<'input>, value: Values<'input>) -> Self {
        MacroDefinition { name, value }
    }
}

impl<'input> ToString for MacroDefinition<'input> {
    fn to_string(&self) -> String {
        return format!("{} {}", self.name.to_string(), self.value.to_string());
    }
}

#[derive(Debug)]
pub enum MacroCondition<'input> {
    // Simple Definition
    Simple(MacroDefinition<'input>),
    // Disjunction Expression (a.k.a. Logical-Or)
    Disjunction {
        operator_range: TokenRange,
        lhs: MacroDefinition<'input>,
        rhs: Box<MacroCondition<'input>>,
    },
    // Conjunction Expression (a.k.a. Logical-And)
    Conjunction {
        operator_range: TokenRange,
        lhs: MacroDefinition<'input>,
        rhs: Box<MacroCondition<'input>>,
    },
}

impl<'input> ToString for MacroCondition<'input> {
    fn to_string(&self) -> String {
        match self {
            MacroCondition::Simple(condition) => condition.to_string(),
            MacroCondition::Disjunction {
                operator_range: _,
                lhs,
                rhs,
            } => format!("{} || {}", lhs.to_string(), rhs.to_string()),
            MacroCondition::Conjunction {
                operator_range: _,
                lhs,
                rhs,
            } => format!("{} && {}", lhs.to_string(), rhs.to_string()),
        }
    }
}

#[derive(Debug)]
pub enum IfDefBranch<'input> {
    ElseIfDef {
        line: u32,
        macro_range: TokenRange,
        condition: MacroCondition<'input>,
        body: Lines<'input>,
        branch: Box<IfDefBranch<'input>>,
    },
    Else {
        line: u32,
        macro_range: TokenRange,
        body: Lines<'input>,
        endif_line: u32,
        endif_macro_range: TokenRange,
    },
    EndIf {
        line: u32,
        macro_range: TokenRange,
    },
}

impl<'input> IfDefBranch<'input> {
    /// Get start line of the branch.
    pub fn get_start_line(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef {
                line,
                macro_range: _,
                condition: _,
                body: _,
                branch: _,
            } => *line,
            IfDefBranch::Else {
                line,
                macro_range: _,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => *line,
            IfDefBranch::EndIf {
                line,
                macro_range: _,
            } => *line,
        }
    }

    /// Get end line of this branch.
    pub fn get_end_line(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef {
                line: _,
                macro_range: _,
                condition: _,
                body: _,
                branch,
            } => branch.get_start_line() - 1,
            IfDefBranch::Else {
                line: _,
                macro_range: _,
                body: _,
                endif_line,
                endif_macro_range: _,
            } => *endif_line - 1,
            // For Endif, the start line and the end line are always the same.
            IfDefBranch::EndIf {
                line,
                macro_range: _,
            } => *line,
        }
    }
}

impl<'input> ToString for IfDefBranch<'input> {
    fn to_string(&self) -> String {
        match self {
            IfDefBranch::ElseIfDef {
                line: _,
                macro_range: _,
                condition,
                body: _,
                branch: _,
            } => {
                format!("#elsifdef {}", condition.to_string())
            }
            IfDefBranch::Else {
                line: _,
                macro_range: _,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => "#else".to_string(),
            IfDefBranch::EndIf {
                line: _,
                macro_range: _,
            } => "#endif".to_string(),
        }
    }
}

vec_wrapper!(IfNotDefClauses, VariableStrings);

#[derive(Debug)]
pub enum IfNotDefBranch<'input> {
    Else {
        line: u32,
        macro_range: TokenRange,
        body: Lines<'input>,
        endif_line: u32,
        endif_macro_range: TokenRange,
    },
    EndIf {
        line: u32,
        macro_range: TokenRange,
    },
}

impl<'input> IfNotDefBranch<'input> {
    /// Get the start line of this branch
    pub fn get_start_line(&self) -> u32 {
        match self {
            IfNotDefBranch::Else {
                line,
                macro_range: _,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => *line,
            IfNotDefBranch::EndIf {
                line,
                macro_range: _,
            } => *line,
        }
    }

    /// Get the end line of this branch.
    pub fn get_end_line(&self) -> u32 {
        match self {
            IfNotDefBranch::Else {
                line: _,
                macro_range: _,
                body: _,
                endif_line,
                endif_macro_range: _,
            } => *endif_line - 1,
            // For EndIf, the start and end lines are always the same.
            IfNotDefBranch::EndIf {
                line,
                macro_range: _,
            } => *line,
        }
    }
}

impl<'input> ToString for IfNotDefBranch<'input> {
    fn to_string(&self) -> String {
        match self {
            IfNotDefBranch::Else {
                line: _,
                macro_range: _,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => "#else".to_string(),
            IfNotDefBranch::EndIf {
                line: _,
                macro_range: _,
            } => "#endif".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum Line<'input> {
    /// NOTE: Comments are not really supported by NSPlug. We have them here
    /// because they might be soon.
    Comment {
        comment: &'input str,
        line: u32,
    },
    Macro {
        macro_type: MacroType<'input>,
        comment: Option<&'input str>,
        line: u32,
    },
    Variable {
        variable: Variable<'input>,
        line: u32,
    },
    Error(u32, u32),
    EndOfLine,
}

impl<'input> ToString for Line<'input> {
    fn to_string(&self) -> String {
        match self {
            Line::Comment { comment, line: _ } => {
                format!("// {comment}")
            }
            Line::Macro {
                macro_type,
                comment,
                line: _,
            } => {
                if let Some(comment) = comment {
                    format!("{} // {comment}", macro_type.to_string())
                } else {
                    macro_type.to_string()
                }
            }
            Line::Variable { variable, line: _ } => variable.to_string(),
            Line::Error(_, _) => "".to_string(),
            Line::EndOfLine => "".to_string(),
        }
    }
}

vec_wrapper!(Lines, Line);

// ----------------------------------------------------------------------------
// Tests
#[cfg(test)]
mod tests {

    use crate::lexers::TokenRange;

    use super::{Value, Values, Variable};

    #[test]
    fn test_values_iterator() {
        let mut values = Values::default();

        values.0.push(Value::String(
            "My name is ",
            TokenRange::new(0, 11).unwrap(),
        ));

        values.0.push(Value::Variable(Variable::Regular {
            text: "NAME",
            range: TokenRange::new(11, 18).unwrap(),
        }));

        for v in &values {
            println!("Value: {v:?}");
        }

        println!("!!Values as string: '''{}'''", values.eval());
    }
}
