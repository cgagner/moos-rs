use crate::lexers::TokenRange;
use crate::vec_wrapper;
use crate::{TreeNode, TreeStr};

#[derive(Debug)]
pub enum Value {
    Boolean(bool, TreeStr, TokenRange),
    Integer(i64, TreeStr, TokenRange),
    Float(f64, TreeStr, TokenRange),
    String(TreeStr, TokenRange),
    Quote(Quote),
    Variable(Variable),
}

impl ToString for Value {
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

impl From<Variable> for Value {
    fn from(value: Variable) -> Self {
        Self::Variable(value)
    }
}

impl TryFrom<Value> for Variable {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Variable(variable) => Ok(variable),
            _ => Err(()),
        }
    }
}

// Declares a new struct Values that wraps a Vec<Value>
vec_wrapper!(Values, Value);

#[derive(Debug, Clone)]
pub enum Variable {
    Regular { text: TreeStr, range: TokenRange },
    Partial { text: TreeStr, range: TokenRange },
}
impl ToString for Variable {
    fn to_string(&self) -> String {
        match self {
            Variable::Regular { text, range: _ } => format!("${{{}}}", text),
            Variable::Partial { text, range: _ } => format!("${{{}", text),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VariableString {
    String(TreeStr, TokenRange),
    Variable(Variable),
}

impl VariableString {
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

impl ToString for VariableString {
    fn to_string(&self) -> String {
        match self {
            Self::String(value_str, _) => (*value_str).to_string(),
            // We won't evaluate plug variables as part of this parser.
            Self::Variable(variable) => variable.to_string(),
        }
    }
}

impl From<Variable> for VariableString {
    fn from(value: Variable) -> Self {
        Self::Variable(value)
    }
}

impl TryFrom<VariableString> for Variable {
    type Error = ();

    fn try_from(value: VariableString) -> Result<Self, Self::Error> {
        match value {
            VariableString::Variable(variable) => Ok(variable),
            _ => Err(()),
        }
    }
}

vec_wrapper!(VariableStrings, VariableString);

#[derive(Debug)]
pub struct Quote {
    pub content: Values,
    pub range: TokenRange,
}

impl ToString for Quote {
    fn to_string(&self) -> String {
        return format!("\"{}\"", self.content.to_string());
    }
}

impl From<Quote> for Value {
    fn from(value: Quote) -> Self {
        Self::Quote(value)
    }
}

#[derive(Debug)]
pub struct Comment {
    pub text: TreeStr,
    pub range: TokenRange,
}

impl ToString for Comment {
    fn to_string(&self) -> String {
        format!("// {}", self.text)
    }
}

#[derive(Debug)]

pub struct Assignment {
    pub name: VariableStrings,
    pub value: Values,
    pub comment: Option<Comment>,
}

impl ToString for Assignment {
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
pub struct ProcessConfig {
    /// Comment at the end of the ProcessConfig line
    pub process_config_comment: Option<Comment>,
    /// Name of the process
    pub process_name: VariableStrings,
    /// Comments between ProcessConfig line and curly brace
    pub prelude_comments: Lines,
    /// Line number for the opening curly brace
    pub open_curly_line: u32,
    /// Line number for the opening curly brace
    pub open_curly_index: u32,
    /// Comment after the open curly brace
    pub open_curly_comment: Option<Comment>,
    /// Line number of the closing curly brace
    pub close_curly_line: u32,
    /// Line number of the closing curly brace
    pub close_curly_index: u32,
    /// Comment after the close curly brace
    pub close_curly_comment: Option<Comment>,
    /// Lines inside of the ProcessConfig block. This should throw an error
    /// if a ProcessConfig is found inside another ProcessConfig
    pub body: Lines,
}

impl ToString for ProcessConfig {
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
pub enum Line {
    Comment {
        comment: Comment,
        line: u32,
    },
    Assignment {
        assignment: Assignment,
        line: u32,
    },
    Define {
        assignment: Assignment,
        line: u32,
        /// Range of the 'define:' keyword
        range: TokenRange,
    },
    ProcessConfig {
        process_config: ProcessConfig,
        /// Line of the ProcessConfig
        line: u32,
        /// Range of the 'ProcessConfig' keyword
        range: TokenRange,
    },
    Variable {
        variable: Variable,
        line: u32,
    },
    Error {
        start_line: u32,
        end_line: u32,
    },
    EndOfLine {
        line: u32,
        index: u32,
    },
}

impl Line {
    pub fn get_line_number(&self) -> u32 {
        match self {
            Line::Comment { comment: _, line } => *line,
            Line::Assignment {
                assignment: _,
                line,
            } => *line,
            Line::Define {
                assignment: _,
                line,
                range: _,
            } => *line,
            Line::ProcessConfig {
                process_config: _,
                line,
                range: _,
            } => *line,
            Line::Variable { variable: _, line } => *line,
            Line::Error {
                start_line,
                end_line: _,
            } => *start_line,
            Line::EndOfLine { line, index: _ } => *line,
        }
    }
}

impl ToString for Line {
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
            Line::Error {
                start_line: _,
                end_line: _,
            } => "".to_string(),
            Line::EndOfLine { line: _, index: _ } => "".to_string(),
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
            "My name is ".into(),
            TokenRange::new(0, 11).unwrap(),
        ));

        values.0.push(Value::Variable(Variable::Regular {
            text: "NAME".into(),
            range: TokenRange::new(11, 18).unwrap(),
        }));

        for v in &values {
            println!("Value: {v:?}");
        }

        println!("!!Values as string: '''{}'''", values.eval());
    }
}
