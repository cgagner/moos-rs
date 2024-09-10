use std::f32::consts::E;

use crate::base::{TreeNode, VariableMarker};
use crate::lexers::TokenRange;
use crate::vec_wrapper;
use crate::TreeStr;

#[derive(Debug)]
pub struct PlugComment;

impl crate::base::CommentMarker for PlugComment {
    const COMMENT_MARKER: &'static str = "//";
}
pub type Comment = crate::base::Comment<PlugComment>;

pub type Quote = crate::base::Quote<Values>;

impl From<Quote> for Value {
    fn from(value: Quote) -> Self {
        Self::Quote(value)
    }
}

pub type Value = crate::base::Value<Variable, Values>;
// Declares a new struct Values that wraps a Vec<Value>
vec_wrapper!(Values, Value);

pub type VariableString = crate::base::VariableString<Variable>;
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
pub struct BehaviorBlock {
    /// Comment at the end of the BehaviorBlock line
    pub behavior_block_comment: Option<Comment>,
    /// Name of the behavior
    pub behavior_name: VariableStrings,
    /// Comments between BehaviorBlock line and curly brace
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
    /// Lines inside of the BehaviorBlock block. This should throw an error
    /// if a BehaviorBlock is found inside another BehaviorBlock
    pub body: Lines,
}

impl TreeNode for BehaviorBlock {
    fn get_start_index(&self) -> u32 {
        self.behavior_name.get_start_index()
    }

    fn get_end_index(&self) -> u32 {
        if let Some(comment) = &self.open_curly_comment {
            comment.get_end_index()
        } else {
            self.open_curly_index
        }
    }
}

impl ToString for BehaviorBlock {
    fn to_string(&self) -> String {
        if let Some(comment) = &self.behavior_block_comment {
            format!(
                "Behavior = {} {}",
                self.behavior_name.to_string(),
                comment.to_string()
            )
        } else {
            format!("Behavior = {}", self.behavior_name.to_string())
        }
    }
}

#[derive(Debug)]
pub struct UnknownBlock {
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
    /// Lines inside of the BehaviorBlock block. This should throw an error
    /// if a BehaviorBlock is found inside another BehaviorBlock
    pub body: Lines,
}

impl TreeNode for UnknownBlock {
    fn get_start_index(&self) -> u32 {
        self.open_curly_index
    }

    fn get_end_index(&self) -> u32 {
        if let Some(comment) = &self.open_curly_comment {
            comment.get_end_index()
        } else {
            self.open_curly_index + 1
        }
    }
}

impl ToString for UnknownBlock {
    fn to_string(&self) -> String {
        "{ .. }".to_string()
    }
}

#[derive(Debug)]
pub struct SetBlock {
    /// Comment at the end of the Set line
    pub set_block_comment: Option<Comment>,
    /// Mode Variable Name
    pub mode_variable_name: VariableStrings,
    /// Mode Value
    pub mode_value: VariableStrings,
    /// Comments between SetBlock line and curly brace
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
    /// Lines inside of the SetBlock block. This should throw an error
    /// if a BehaviorBlock is found inside another SetBlock
    pub body: Lines,
    /// Else Value
    pub else_value: Option<VariableStrings>,
}

impl TreeNode for SetBlock {
    fn get_start_index(&self) -> u32 {
        self.mode_variable_name.get_start_index()
    }

    fn get_end_index(&self) -> u32 {
        if let Some(comment) = &self.open_curly_comment {
            comment.get_end_index()
        } else {
            self.open_curly_index
        }
    }
}

impl ToString for SetBlock {
    fn to_string(&self) -> String {
        if let Some(comment) = &self.set_block_comment {
            format!(
                "set {} = {} {}",
                self.mode_variable_name.to_string(),
                self.mode_value.to_string(),
                comment.to_string()
            )
        } else {
            format!(
                "set {} = {}",
                self.mode_variable_name.to_string(),
                self.mode_value.to_string()
            )
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
    Initialize {
        assignment: Assignment,
        deferred: bool,
        line: u32,
        /// Range of the 'initialize:' keyword
        range: TokenRange,
    },
    BehaviorBlock {
        behavior_block: BehaviorBlock,
        /// Line of the BehaviorBlock
        line: u32,
        /// Range of the 'Behavior' keyword
        range: TokenRange,
    },
    UnknownBlock {
        unknown_block: UnknownBlock,
        /// Line of the BehaviorBlock
        line: u32,
        /// Range of the open curly brace
        range: TokenRange,
    },
    SetBlock {
        set_block: SetBlock,
        /// Line of the SetBlock
        line: u32,
        /// Range of the 'Set' keyword
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
            Line::Initialize {
                assignment: _,
                deferred: _,
                line,
                range: _,
            } => *line,
            Line::BehaviorBlock {
                behavior_block: _,
                line,
                range: _,
            } => *line,
            Line::UnknownBlock {
                unknown_block: _,
                line,
                range: _,
            } => *line,
            Line::SetBlock {
                set_block: _,
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
            Line::Initialize {
                assignment: _,
                deferred: _,
                line: _,
                range,
            } => range.start,
            Line::BehaviorBlock {
                behavior_block: _,
                line: _,
                range,
            } => range.start,
            Line::UnknownBlock {
                unknown_block: _,
                line: _,
                range,
            } => range.start,
            Line::SetBlock {
                set_block: _,
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
            Line::Initialize {
                assignment,
                deferred: _,
                line: _,
                range: _,
            } => assignment.get_end_index(),
            Line::BehaviorBlock {
                behavior_block,
                line: _,
                range: _,
            } => behavior_block.get_end_index(),
            Line::UnknownBlock {
                unknown_block,
                line: _,
                range: _,
            } => unknown_block.get_end_index(),
            Line::SetBlock {
                set_block,
                line: _,
                range: _,
            } => set_block.get_end_index(),
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
            Line::Initialize {
                assignment,
                deferred,
                line: _,
                range: _,
            } => {
                if *deferred {
                    format!("initialize_ {}", assignment.to_string())
                } else {
                    format!("initialize {}", assignment.to_string())
                }
            }
            Line::BehaviorBlock {
                behavior_block,
                line: _,
                range: _,
            } => behavior_block.to_string(),
            Line::UnknownBlock {
                unknown_block,
                line: _,
                range: _,
            } => unknown_block.to_string(),
            Line::SetBlock {
                set_block,
                line: _,
                range: _,
            } => set_block.to_string(),
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
