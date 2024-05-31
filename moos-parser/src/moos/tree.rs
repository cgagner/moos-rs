use crate::lexers::TokenRange;
use crate::{vec_wrapper, VariableMarker};
use crate::{TreeNode, TreeStr};

#[derive(Debug)]
pub struct PlugComment;

impl crate::CommentMarker for PlugComment {
    const COMMENT_MARKER: &'static str = "//";
}
pub type Comment = crate::Comment<PlugComment>;

pub type Quote = crate::Quote<Values>;

impl From<Quote> for Value {
    fn from(value: Quote) -> Self {
        Self::Quote(value)
    }
}

pub type Value = crate::Value<Variable, Values>;
// Declares a new struct Values that wraps a Vec<Value>
vec_wrapper!(Values, Value);

pub type VariableString = crate::VariableString<Variable>;
vec_wrapper!(VariableStrings, VariableString);

#[derive(Debug, Clone)]
pub enum Variable {
    Regular { text: TreeStr, range: TokenRange },
    Partial { text: TreeStr, range: TokenRange },
}

impl Variable {
    pub fn get_text(&self) -> &str {
        match self {
            Variable::Regular { text, range: _ } | Variable::Partial { text, range: _ } => text,
        }
    }
}

impl VariableMarker for Variable {
    fn get_token_range(&self) -> &TokenRange {
        match self {
            Variable::Regular { text: _, range } | Variable::Partial { text: _, range } => range,
        }
    }
}

impl TreeNode for Variable {
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    #[inline]
    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl ToString for Variable {
    fn to_string(&self) -> String {
        match self {
            Variable::Regular { text, range: _ } => format!("${{{}}}", text),
            Variable::Partial { text, range: _ } => format!("${{{}", text),
        }
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

impl TryFrom<VariableString> for Variable {
    type Error = ();

    fn try_from(value: VariableString) -> Result<Self, Self::Error> {
        match value {
            VariableString::Variable(variable) => Ok(variable),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]

pub struct Assignment {
    pub name: VariableStrings,
    pub value: Values,
    pub comment: Option<Comment>,
}

impl TreeNode for Assignment {
    fn get_start_index(&self) -> u32 {
        self.name.get_start_index()
    }

    fn get_end_index(&self) -> u32 {
        if let Some(comment) = &self.comment {
            comment.get_end_index()
        } else if !self.value.is_empty() {
            self.value.get_end_index()
        } else {
            self.name.get_end_index()
        }
    }
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

impl TreeNode for ProcessConfig {
    fn get_start_index(&self) -> u32 {
        self.process_name.get_start_index()
    }

    fn get_end_index(&self) -> u32 {
        if let Some(comment) = &self.open_curly_comment {
            comment.get_end_index()
        } else {
            self.open_curly_index
        }
    }
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

impl TreeNode for Line {
    fn get_start_index(&self) -> u32 {
        match self {
            Line::Comment { comment, line: _ } => comment.get_start_index(),
            Line::Assignment {
                assignment,
                line: _,
            } => assignment.get_start_index(),
            Line::Define {
                assignment: _,
                line: _,
                range,
            } => range.start,
            Line::ProcessConfig {
                process_config: _,
                line: _,
                range,
            } => range.start,
            Line::Variable { variable, line: _ } => variable.get_start_index(),
            Line::Error {
                start_line: _,
                end_line: _,
            } => 0,
            Line::EndOfLine { line: _, index } => *index,
        }
    }

    fn get_end_index(&self) -> u32 {
        match self {
            Line::Comment { comment, line: _ } => comment.get_end_index(),
            Line::Assignment {
                assignment,
                line: _,
            } => assignment.get_end_index(),
            Line::Define {
                assignment,
                line: _,
                range: _,
            } => assignment.get_end_index(),
            Line::ProcessConfig {
                process_config,
                line: _,
                range: _,
            } => process_config.get_end_index(),
            Line::Variable { variable, line: _ } => variable.get_end_index(),
            Line::Error {
                start_line: _,
                end_line: _,
            } => 0,
            Line::EndOfLine { line: _, index } => *index,
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
