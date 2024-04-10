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
    Partial {
        text: &'input str,
        range: TokenRange,
    },
}
impl<'input> ToString for Variable<'input> {
    fn to_string(&self) -> String {
        match self {
            Variable::Regular { text, range: _ } => format!("${{{}}}", text),
            Variable::Partial { text, range: _ } => format!("${{{}", text),
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
    pub content: Values<'input>,
    pub range: TokenRange,
}

impl<'input> ToString for Quote<'input> {
    fn to_string(&self) -> String {
        return format!("\"{}\"", self.content.to_string());
    }
}

impl<'input> From<Quote<'input>> for Value<'input> {
    fn from(value: Quote<'input>) -> Self {
        Self::Quote(value)
    }
}

#[derive(Debug)]
pub struct Comment<'input> {
    pub text: &'input str,
    pub range: TokenRange,
}

impl<'input> ToString for Comment<'input> {
    fn to_string(&self) -> String {
        format!("// {}", self.text)
    }
}

#[derive(Debug)]

pub struct Assignment<'input> {
    pub name: VariableStrings<'input>,
    pub value: Values<'input>,
    pub comment: Option<Comment<'input>>,
}

impl<'input> ToString for Assignment<'input> {
    fn to_string(&self) -> String {
        if let Some(comment) = &self.comment {
            format!(
                "{} = {} {}",
                self.name.to_string(),
                self.value.to_string(),
                comment.to_string(),
            )
        } else {
            format!("{} = {}", self.name.to_string(), self.value.to_string())
        }
    }
}

#[derive(Debug)]
pub struct ProcessConfig<'input> {
    /// Comment at the end of the ProcessConfig line
    pub process_config_comment: Option<Comment<'input>>,
    /// Name of the process
    pub process_name: VariableStrings<'input>,
    /// Comments between ProcessConfig line and curly brace
    pub prelude_comments: Lines<'input>,
    /// Line number for the opening curly brace
    pub open_curly_line: u32,
    /// Line number for the opening curly brace
    pub open_curly_index: u32,
    /// Comment after the open curly brace
    pub open_curly_comment: Option<Comment<'input>>,
    /// Line number of the closing curly brace
    pub close_curly_line: u32,
    /// Line number of the closing curly brace
    pub close_curly_index: u32,
    /// Comment after the close curly brace
    pub close_curly_comment: Option<Comment<'input>>,
    /// Lines inside of the ProcessConfig block. This should throw an error
    /// if a ProcessConfig is found inside another ProcessConfig
    pub body: Lines<'input>,
}

impl<'input> ToString for ProcessConfig<'input> {
    fn to_string(&self) -> String {
        if let Some(comment) = &self.process_config_comment {
            format!(
                "ProcessConfig = {} {}",
                self.process_name.to_string(),
                comment.to_string()
            )
        } else {
            format!("ProcessConfig = {}", self.process_name.to_string())
        }
    }
}

#[derive(Debug)]
pub enum Line<'input> {
    Comment {
        comment: Comment<'input>,
        line: u32,
    },
    Assignment {
        assignment: Assignment<'input>,
        line: u32,
    },
    Define {
        assignment: Assignment<'input>,
        line: u32,
        /// Range of the 'define:' keyword
        range: TokenRange,
    },
    ProcessConfig {
        process_config: ProcessConfig<'input>,
        /// Line of the ProcessConfig
        line: u32,
        /// Range of the 'ProcessConfig' keyword
        range: TokenRange,
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
            Line::Comment { comment, line: _ } => comment.to_string(),
            Line::Assignment {
                assignment,
                line: _,
            } => assignment.to_string(),
            Line::Define {
                assignment,
                line: _,
                range: _,
            } => {
                format!("define: {}", assignment.to_string())
            }
            Line::ProcessConfig {
                process_config,
                line: _,
                range: _,
            } => process_config.to_string(),
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
