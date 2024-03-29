use crate::lexers::TokenRange;
use crate::vec_wrapper;
use lalrpop_util::lalrpop_mod;
use lalrpop_util::ErrorRecovery;

lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub moos
); // synthesized by LALRPOP

/// TODO: Dear Future Chris: Please fix this enumeration. This should be able
/// to handle any of the tokens that can compose a value. Additionally, there
/// needs to be a collection of this enum that implements the eval method
/// to get a string.
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

#[derive(Debug)]
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
            Variable::Regular { text, range } => format!("$({})", text),
            Variable::Upper { text, range } => format!("%({})", text),
            Variable::Partial { text, range } => format!("$({}", text),
            Variable::PartialUpper { text, range } => format!("%({}", text),
        }
    }
}

#[derive(Debug)]
pub enum VariableString<'input> {
    String(&'input str, TokenRange),
    Variable(Variable<'input>),
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

#[derive(Debug)]
pub enum MacroType<'input> {
    Define {
        definition: MacroDefinition<'input>,
        /// Range of the "#define"
        range: TokenRange,
    },
    Include {
        path: IncludePath<'input>,
        /// Range of the "#include"
        range: TokenRange,
    },
    IfDef {
        /// Range of the "#ifdef"
        range: TokenRange,
    },
    IfNotDef {
        /// Range of the "#ifndef"
        range: TokenRange,
    },
    ElseIfDef {
        /// Range of the "#elseifdef"
        range: TokenRange,
    },
    Else {
        /// Range of the "#else"
        range: TokenRange,
    },
    EndIf {
        /// Range of the "#endif"
        range: TokenRange,
    },
}

#[derive(Debug)]
pub struct MacroDefinition<'input> {
    name: Values<'input>,
    value: Values<'input>,
}

impl<'input> MacroDefinition<'input> {
    /// Create a new MacroDefinition
    pub fn new(name: Values<'input>, value: Values<'input>) -> Self {
        MacroDefinition { name, value }
    }

    pub fn eval() -> bool {
        // TODO: Implement
        false
    }
}

#[derive(Debug)]
pub enum MacroCondition<'input> {
    // Simple Definition
    Simple(MacroDefinition<'input>),
    // Disjunction Expression (a.k.a. Logical-Or)
    Disjunction(Vec<MacroDefinition<'input>>),
    // Conjunction Expression (a.k.a. Logical-And)
    Conjunction(Vec<MacroDefinition<'input>>),
    // Mixture of Disjunction and Conjunction - This is an error or false
    Mixed(Vec<MacroDefinition<'input>>),
}

#[derive(Debug)]
pub enum Line<'input> {
    Comment {
        comment: &'input str,
        line: u32,
    },
    Define {
        name: &'input str,
        value: Value<'input>,
        comment: Option<&'input str>,
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
    Error,
    EndOfLine,
}

// ----------------------------------------------------------------------------
// Tests
#[cfg(test)]
mod tests {

    use crate::{
        lexer::{Lexer, State},
        lexers::TokenRange,
    };

    use super::{Value, Values, Variable};

    lalrpop_mod!(
        #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
        pub moos
    ); // syntesized by LALRPOP

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
