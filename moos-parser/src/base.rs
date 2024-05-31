use std::marker::PhantomData;

use crate::lexers;
use crate::lexers::TokenRange;
use crate::TreeStr;

pub trait TreeNode: ToString {
    /// Get the range in the current line for the node
    #[inline]
    fn get_range(&self) -> lexers::TokenRange {
        lexers::TokenRange {
            start: self.get_start_index(),
            end: self.get_end_index(),
        }
    }

    /// Get the start index in the current line for the node
    fn get_start_index(&self) -> u32;

    /// Get the end index in the current line for the node
    fn get_end_index(&self) -> u32;

    /// Check if the specified index is inside the range of this node.
    /// This will return true if the index is equal to the start or end index
    /// as well.
    #[inline]
    fn is_inside(&self, index: u32) -> bool {
        index >= self.get_start_index() && index <= self.get_end_index()
    }
}

pub trait CommentMarker {
    const COMMENT_MARKER: &'static str;
}

#[derive(Debug)]
pub struct Comment<T: CommentMarker> {
    pub text: TreeStr,
    pub range: TokenRange,
    _phantom: PhantomData<T>,
}

impl<T: CommentMarker> Comment<T> {
    pub fn new(text: TreeStr, range: TokenRange) -> Self {
        Self {
            text,
            range,
            _phantom: PhantomData::default(),
        }
    }

    /// Get the range in the line for the Comment
    #[inline]
    pub fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<T: CommentMarker> TreeNode for Comment<T> {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl<T: CommentMarker> ToString for Comment<T> {
    fn to_string(&self) -> String {
        format!("{} {}", T::COMMENT_MARKER, self.text)
    }
}

#[derive(Debug)]
pub struct Quote<V: ToString> {
    pub content: V,
    pub range: TokenRange,
}

impl<V: ToString> Quote<V> {
    pub fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<V: ToString> TreeNode for Quote<V> {
    /// Get the start index in the line for the value
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.range.start
    }

    /// Get the end index in the line for the value
    #[inline]
    fn get_end_index(&self) -> u32 {
        self.range.end
    }
}

impl<V: ToString> ToString for Quote<V> {
    fn to_string(&self) -> String {
        return format!("\"{}\"", self.content.to_string());
    }
}

pub trait VariableMarker: ToString {
    fn get_token_range(&self) -> &TokenRange;
}

#[derive(Debug)]
pub enum Value<V: VariableMarker, QV: ToString> {
    Boolean(bool, TreeStr, TokenRange),
    Integer(i64, TreeStr, TokenRange),
    Float(f64, TreeStr, TokenRange),
    String(TreeStr, TokenRange),
    Quote(Quote<QV>),
    Variable(V),
}

impl<V: VariableMarker, QV: ToString> Value<V, QV> {
    /// Get the range in the line for the value
    #[inline]
    fn get_token_range(&self) -> &TokenRange {
        match self {
            Value::Boolean(_, _, range) => range,
            Value::Integer(_, _, range) => range,
            Value::Float(_, _, range) => range,
            Value::String(_, range) => range,
            Value::Quote(quote) => quote.get_token_range(),
            Value::Variable(variable) => variable.get_token_range(),
        }
    }
}

impl<V: VariableMarker, QV: ToString> TreeNode for Value<V, QV> {
    /// Get the start index in the line for the value
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    /// Get the end index in the line for the value
    #[inline]
    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl<V: VariableMarker, QV: ToString> ToString for Value<V, QV> {
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

impl<V: VariableMarker, QV: ToString> From<V> for Value<V, QV> {
    fn from(value: V) -> Self {
        Self::Variable(value)
    }
}

#[derive(Debug, Clone)]
pub enum VariableString<V: VariableMarker> {
    String(TreeStr, TokenRange),
    Variable(V),
}

impl<V: VariableMarker> VariableString<V> {
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

    /// Get the range of the VariableString in the line
    #[inline]
    fn get_token_range(&self) -> &TokenRange {
        match self {
            VariableString::String(_, range) => range,
            VariableString::Variable(variable) => variable.get_token_range(),
        }
    }
}

impl<V: VariableMarker> TreeNode for VariableString<V> {
    /// Get the start index in the line for the value
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    /// Get the end index in the line for the value
    #[inline]
    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl<V: VariableMarker> ToString for VariableString<V> {
    fn to_string(&self) -> String {
        match self {
            Self::String(value_str, _) => (*value_str).to_string(),
            // We won't evaluate plug variables as part of this parser.
            Self::Variable(variable) => variable.to_string(),
        }
    }
}

impl<V: VariableMarker, QV: ToString> From<Value<V, QV>> for VariableString<V> {
    fn from(value: Value<V, QV>) -> Self {
        match value {
            Value::Boolean(_, text, range)
            | Value::Integer(_, text, range)
            | Value::Float(_, text, range)
            | Value::String(text, range) => VariableString::String(text, range),
            Value::Quote(quote) => {
                tracing::error!(
                    "Calling From<Value> to VariableString with a Quote is not supported."
                );
                VariableString::String("".into(), quote.range)
            }
            Value::Variable(variable) => VariableString::Variable(variable),
        }
    }
}

impl<V: VariableMarker> From<V> for VariableString<V> {
    fn from(value: V) -> Self {
        Self::Variable(value)
    }
}
