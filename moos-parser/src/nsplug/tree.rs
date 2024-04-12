use crate::lexers::TokenRange;
use crate::vec_wrapper;
use crate::TreeNode;

#[derive(Debug)]
pub enum Value<'input> {
    Boolean(bool, &'input str, TokenRange),
    Integer(i64, &'input str, TokenRange),
    Float(f64, &'input str, TokenRange),
    String(&'input str, TokenRange),
    Quote(Quote<'input>),
    Variable(Variable<'input>),
}

impl<'input> Value<'input> {
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

impl<'input> TreeNode for Value<'input> {
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

impl<'input> Variable<'input> {
    pub fn get_text(&self) -> &str {
        match self {
            Variable::Regular { text, range: _ }
            | Variable::Upper { text, range: _ }
            | Variable::Partial { text, range: _ }
            | Variable::PartialUpper { text, range: _ } => text,
        }
    }

    fn get_token_range(&self) -> &TokenRange {
        match self {
            Variable::Regular { text: _, range }
            | Variable::Upper { text: _, range }
            | Variable::Partial { text: _, range }
            | Variable::PartialUpper { text: _, range } => range,
        }
    }
}

impl<'input> TreeNode for Variable<'input> {
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    #[inline]
    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
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

    /// Get the range of the VariableString in the line
    #[inline]
    fn get_token_range(&self) -> &TokenRange {
        match self {
            VariableString::String(_, range) => range,
            VariableString::Variable(variable) => variable.get_token_range(),
        }
    }
}

impl<'input> TreeNode for VariableString<'input> {
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

impl<'input> TreeNode for VariableStrings<'input> {
    fn get_start_index(&self) -> u32 {
        if let Some(v) = self.0.first() {
            v.get_start_index()
        } else {
            0
        }
    }

    fn get_end_index(&self) -> u32 {
        if let Some(v) = self.0.last() {
            v.get_end_index()
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub struct Quote<'input> {
    pub content: Values<'input>,
    pub range: TokenRange,
}

impl<'input> Quote<'input> {
    fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<'input> TreeNode for Quote<'input> {
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

impl<'input> Comment<'input> {
    /// Get the range in the line for the Comment
    #[inline]
    pub fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<'input> TreeNode for Comment<'input> {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl<'input> ToString for Comment<'input> {
    fn to_string(&self) -> String {
        format!("// {}", self.text)
    }
}

#[derive(Debug)]
pub enum IncludePath<'input> {
    VariableStrings(VariableStrings<'input>, TokenRange),
    Quote(Quote<'input>),
}

impl<'input> IncludePath<'input> {
    /// Get the range in the line for the IncludePath
    #[inline]
    pub fn get_token_range(&self) -> &TokenRange {
        match self {
            IncludePath::VariableStrings(_, range) => range,
            IncludePath::Quote(quote) => quote.get_token_range(),
        }
    }
}

impl<'input> TreeNode for IncludePath<'input> {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
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

    /// Get the range in the line for the Include Tag
    #[inline]
    fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<'input> TreeNode for IncludeTag<'input> {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
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

impl<'input> MacroType<'input> {
    /// Get the TokenRange for the Macro keyword.
    fn get_token_range(&self) -> &TokenRange {
        match self {
            MacroType::Define {
                definition: _,
                range,
            }
            | MacroType::Include {
                path: _,
                tag: _,
                range,
            }
            | MacroType::IfDef {
                condition: _,
                branch: _,
                body: _,
                range,
            }
            | MacroType::IfNotDef {
                clauses: _,
                branch: _,
                body: _,
                range,
            } => range,
        }
    }

    #[cfg(feature = "lsp-types")]
    /// Create TextEdits for the macros. This should only manipulate the
    /// whitespace in the line.
    pub fn format(
        &self,
        line: u32,
        line_end_index: u32,
        format_options: &FormatOptions,
        level: u32,
    ) -> Vec<lsp_types::TextEdit> {
        use lsp_types::Position;

        let mut lines = Vec::new();

        /// TODO: Need to figure out how to handle tabs
        let start_index = format_options.tab_size * level;

        let new_text = self.to_string();

        if start_index != self.get_start_index()
            || (start_index + new_text.len() as u32) != line_end_index
        {
            tracing::info!("Formatting line({line}): '{new_text}'");
            lines.push(lsp_types::TextEdit {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line,
                        character: start_index,
                    },
                    end: lsp_types::Position {
                        line: line,
                        character: line_end_index,
                    },
                },
                new_text,
            });
        }

        /// TODO: Need to handle formats recursively
        return lines;
    }
}

impl<'input> TreeNode for MacroType<'input> {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        match self {
            MacroType::Define {
                definition,
                range: _,
            } => definition.get_end_index(),
            MacroType::Include {
                path,
                tag,
                range: _,
            } => {
                if let Some(tag) = tag {
                    tag.get_end_index()
                } else {
                    path.get_end_index()
                }
            }
            MacroType::IfDef {
                condition,
                branch: _,
                body: _,
                range: _,
            } => condition.get_end_index(),
            MacroType::IfNotDef {
                clauses,
                branch: _,
                body: _,
                range: _,
            } => clauses.get_end_index(),
        }
    }
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

impl<'input> TreeNode for MacroDefinition<'input> {
    #[inline]
    fn get_start_index(&self) -> u32 {
        if let Some(v) = self.name.first() {
            v.get_start_index()
        } else {
            0
        }
    }

    #[inline]
    fn get_end_index(&self) -> u32 {
        if let Some(v) = self.value.last() {
            v.get_end_index()
        } else if let Some(v) = self.name.first() {
            v.get_end_index()
        } else {
            0
        }
    }
}

impl<'input> ToString for MacroDefinition<'input> {
    fn to_string(&self) -> String {
        if self.value.is_empty() {
            return format!("{}", self.name.to_string());
        } else {
            return format!("{} {}", self.name.to_string(), self.value.to_string());
        }
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

impl<'input> TreeNode for MacroCondition<'input> {
    #[inline]
    fn get_start_index(&self) -> u32 {
        match self {
            MacroCondition::Simple(def) => def.get_start_index(),
            MacroCondition::Disjunction {
                operator_range: _,
                lhs,
                rhs: _,
            } => lhs.get_start_index(),
            MacroCondition::Conjunction {
                operator_range: _,
                lhs,
                rhs: _,
            } => lhs.get_start_index(),
        }
    }

    #[inline]
    fn get_end_index(&self) -> u32 {
        match self {
            MacroCondition::Simple(def) => def.get_end_index(),
            MacroCondition::Disjunction {
                operator_range: _,
                lhs: _,
                rhs,
            } => rhs.get_end_index(),
            MacroCondition::Conjunction {
                operator_range: _,
                lhs: _,
                rhs,
            } => rhs.get_end_index(),
        }
    }
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

impl<'input> TreeNode for IfDefBranch<'input> {
    fn get_start_index(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef {
                line: _,
                macro_range,
                condition: _,
                body: _,
                branch: _,
            } => macro_range.start,
            IfDefBranch::Else {
                line: _,
                macro_range,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => macro_range.start,
            IfDefBranch::EndIf {
                line: _,
                macro_range,
            } => macro_range.start,
        }
    }

    fn get_end_index(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef {
                line: _,
                macro_range: _,
                condition,
                body: _,
                branch: _,
            } => condition.get_end_index(),
            IfDefBranch::Else {
                line: _,
                macro_range,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => macro_range.end,
            IfDefBranch::EndIf {
                line: _,
                macro_range,
            } => macro_range.end,
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

impl<'input> TreeNode for IfNotDefClauses<'input> {
    fn get_start_index(&self) -> u32 {
        if let Some(v) = self.0.first() {
            v.get_start_index()
        } else {
            0
        }
    }

    fn get_end_index(&self) -> u32 {
        if let Some(v) = self.0.last() {
            v.get_end_index()
        } else {
            0
        }
    }
}

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

impl<'input> TreeNode for IfNotDefBranch<'input> {
    fn get_start_index(&self) -> u32 {
        match self {
            IfNotDefBranch::Else {
                line: _,
                macro_range,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => macro_range.start,
            IfNotDefBranch::EndIf {
                line: _,
                macro_range,
            } => macro_range.start,
        }
    }

    fn get_end_index(&self) -> u32 {
        match self {
            IfNotDefBranch::Else {
                line: _,
                macro_range,
                body: _,
                endif_line: _,
                endif_macro_range: _,
            } => macro_range.end,
            IfNotDefBranch::EndIf {
                line: _,
                macro_range,
            } => macro_range.end,
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
        comment: Comment<'input>,
        line: u32,
        line_end_index: u32,
    },
    Macro {
        macro_type: MacroType<'input>,
        comment: Option<Comment<'input>>,
        line: u32,
        line_end_index: u32,
    },
    Variable {
        variable: Variable<'input>,
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

pub struct FormatOptions {
    pub insert_spaces: bool,
    pub tab_size: u32,
}

#[cfg(feature = "lsp-types")]
impl From<lsp_types::FormattingOptions> for FormatOptions {
    fn from(value: lsp_types::FormattingOptions) -> Self {
        FormatOptions {
            insert_spaces: value.insert_spaces,
            tab_size: value.tab_size,
        }
    }
}

#[cfg(feature = "lsp-types")]
impl From<&lsp_types::FormattingOptions> for FormatOptions {
    fn from(value: &lsp_types::FormattingOptions) -> Self {
        FormatOptions {
            insert_spaces: value.insert_spaces,
            tab_size: value.tab_size,
        }
    }
}

impl<'input> Line<'input> {
    pub fn get_line_number(&self) -> u32 {
        match self {
            Line::Comment {
                comment: _,
                line,
                line_end_index: _,
            } => *line,
            Line::Macro {
                macro_type: _,
                comment: _,
                line,
                line_end_index: _,
            } => *line,
            Line::Variable { variable: _, line } => *line,
            Line::Error {
                start_line,
                end_line: _,
            } => *start_line,
            Line::EndOfLine { line, index: _ } => *line,
        }
    }

    #[cfg(feature = "lsp-types")]
    pub fn format(&self, format_options: &FormatOptions, level: u32) -> Vec<lsp_types::TextEdit> {
        match self {
            Line::Macro {
                macro_type,
                comment: _,
                line,
                line_end_index,
            } => {
                // TODO: Handle comments, though they are note really supported
                return macro_type.format(*line, *line_end_index, format_options, level);
            }
            _ => return Vec::new(),
        }
    }
}

impl<'input> ToString for Line<'input> {
    fn to_string(&self) -> String {
        match self {
            Line::Comment {
                comment,
                line: _,
                line_end_index: _,
            } => comment.to_string(),
            Line::Macro {
                macro_type,
                comment,
                line: _,
                line_end_index: _,
            } => {
                if let Some(comment) = comment {
                    format!("{} {}", macro_type.to_string(), comment.to_string())
                } else {
                    macro_type.to_string()
                }
            }
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
