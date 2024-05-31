use crate::vec_wrapper;
#[cfg(feature = "lsp-types")]
use crate::{create_text_edit, TextFormatter};
use crate::{lexers::TokenRange, VariableMarker};
use crate::{FormatOptions, TreeNode, TreeStr};

pub const DEFINE_STR: &str = "#define";
pub const INCLUDE_STR: &str = "#include";
pub const IFDEF_STR: &str = "#ifdef";
pub const IFNDEF_STR: &str = "#ifndef";
pub const ELSEIFDEF_STR: &str = "#elseifdef";
pub const ELSE_STR: &str = "#else";
pub const ENDIF_STR: &str = "#endif";

#[derive(Debug, Default)]
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
    Upper { text: TreeStr, range: TokenRange },
    Partial { text: TreeStr, range: TokenRange },
    PartialUpper { text: TreeStr, range: TokenRange },
}

impl Variable {
    pub fn get_text(&self) -> &str {
        match self {
            Variable::Regular { text, range: _ }
            | Variable::Upper { text, range: _ }
            | Variable::Partial { text, range: _ }
            | Variable::PartialUpper { text, range: _ } => text,
        }
    }
}

impl VariableMarker for Variable {
    fn get_token_range(&self) -> &TokenRange {
        match self {
            Variable::Regular { text: _, range }
            | Variable::Upper { text: _, range }
            | Variable::Partial { text: _, range }
            | Variable::PartialUpper { text: _, range } => range,
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
            Variable::Regular { text, range: _ } => format!("$({})", text),
            Variable::Upper { text, range: _ } => format!("%({})", text),
            Variable::Partial { text, range: _ } => format!("$({}", text),
            Variable::PartialUpper { text, range: _ } => format!("%({}", text),
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
pub enum IncludePath {
    VariableStrings(VariableStrings, TokenRange),
    Quote(Quote),
}

impl IncludePath {
    /// Get the range in the line for the IncludePath
    #[inline]
    pub fn get_token_range(&self) -> &TokenRange {
        match self {
            IncludePath::VariableStrings(_, range) => range,
            IncludePath::Quote(quote) => quote.get_token_range(),
        }
    }
}

impl TreeNode for IncludePath {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl ToString for IncludePath {
    fn to_string(&self) -> String {
        match self {
            Self::VariableStrings(values, _) => values.to_string(),
            // We won't evaluate plug variables as part of this parser.
            Self::Quote(quote) => quote.to_string(),
        }
    }
}

impl From<Quote> for IncludePath {
    fn from(value: Quote) -> Self {
        Self::Quote(value)
    }
}

#[derive(Debug, Clone)]
pub struct IncludeTag {
    pub tag: TreeStr,
    /// Range of the Include tag. THis includes the start and ending brackets
    pub range: TokenRange,
}

impl IncludeTag {
    pub fn new(tag: TreeStr, range: TokenRange) -> Self {
        Self { tag, range }
    }

    /// Get the range in the line for the Include Tag
    #[inline]
    fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl TreeNode for IncludeTag {
    fn get_start_index(&self) -> u32 {
        self.get_token_range().start
    }

    fn get_end_index(&self) -> u32 {
        self.get_token_range().end
    }
}

impl ToString for IncludeTag {
    fn to_string(&self) -> String {
        format!("<{}>", self.tag)
    }
}

#[derive(Debug)]
pub enum MacroType {
    Define {
        definition: MacroDefinition,
        /// Range of the "#define"
        range: TokenRange,
    },
    Include {
        path: IncludePath,
        /// Optional include tag. Added in 2020.
        tag: Option<IncludeTag>,
        /// Range of the "#include"
        range: TokenRange,
    },
    IfDef {
        condition: MacroCondition,
        branch: IfDefBranch,
        body: Lines,
        /// Range of the "#ifdef"
        range: TokenRange,
    },
    IfNotDef {
        clauses: IfNotDefClauses,
        branch: IfNotDefBranch,
        body: Lines,
        /// Range of the "#ifndef"
        range: TokenRange,
    },
}

impl MacroType {
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
        current_indent: &str,
        format_options: &FormatOptions,
        level: u32,
    ) -> Vec<lsp_types::TextEdit> {
        let mut lines = Vec::new();

        let indent = if format_options.insert_spaces {
            format!("{: ^1$}", "", (level * format_options.tab_size) as usize)
        } else {
            format!("{:\t^1$}", "", level as usize)
        };

        let start_index = if format_options.insert_spaces {
            format_options.tab_size * level
        } else {
            level
        };

        let new_text = self.to_string();

        if current_indent != indent
            || start_index != self.get_start_index()
            || (start_index + new_text.len() as u32) != line_end_index
        {
            let new_text = indent + new_text.as_str();
            tracing::info!("Formatting line({line}): '{new_text}'");
            lines.push(create_text_edit(new_text, line, 0, line_end_index));
        }

        match self {
            MacroType::IfDef {
                condition: _,
                branch,
                body,
                range: _,
            } => {
                //
                lines.extend(
                    body.iter()
                        .flat_map(|line| line.format(format_options, level + 1)),
                );
                lines.extend(branch.format(format_options, level));
            }
            MacroType::IfNotDef {
                clauses: _,
                branch,
                body,
                range: _,
            } => {
                lines.extend(
                    body.iter()
                        .flat_map(|line| line.format(format_options, level + 1)),
                );
                lines.extend(branch.format(format_options, level));
            }
            _ => {}
        }

        return lines;
    }
}

impl TreeNode for MacroType {
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

impl ToString for MacroType {
    fn to_string(&self) -> String {
        match self {
            MacroType::Define {
                definition,
                range: _,
            } => {
                format!("{DEFINE_STR} {}", definition.to_string())
            }
            MacroType::Include {
                path,
                tag,
                range: _,
            } => {
                if let Some(tag) = tag {
                    format!("{INCLUDE_STR} {} {}", path.to_string(), tag.to_string())
                } else {
                    format!("{INCLUDE_STR} {}", path.to_string())
                }
            }
            MacroType::IfDef {
                condition,
                branch: _,
                body: _,
                range: _,
            } => {
                format!("{IFDEF_STR} {}", condition.to_string())
            }
            MacroType::IfNotDef {
                clauses,
                branch: _,
                body: _,
                range: _,
            } => {
                let rtn = IFNDEF_STR.to_string();
                clauses
                    .iter()
                    .fold(rtn, |acc, v| acc + " " + v.to_string().as_str())
            }
        }
    }
}

#[derive(Debug)]
pub struct MacroDefinition {
    pub name: VariableStrings,
    pub value: Values,
}

impl MacroDefinition {
    /// Create a new MacroDefinition
    pub fn new(name: VariableStrings, value: Values) -> Self {
        MacroDefinition { name, value }
    }
}

impl TreeNode for MacroDefinition {
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

impl ToString for MacroDefinition {
    fn to_string(&self) -> String {
        if self.value.is_empty() {
            return format!("{}", self.name.to_string());
        } else {
            return format!("{} {}", self.name.to_string(), self.value.to_string());
        }
    }
}

#[derive(Debug)]
pub enum MacroCondition {
    // Simple Definition
    Simple(MacroDefinition),
    // Disjunction Expression (a.k.a. Logical-Or)
    Disjunction {
        operator_range: TokenRange,
        lhs: MacroDefinition,
        rhs: Box<MacroCondition>,
    },
    // Conjunction Expression (a.k.a. Logical-And)
    Conjunction {
        operator_range: TokenRange,
        lhs: MacroDefinition,
        rhs: Box<MacroCondition>,
    },
}

impl MacroCondition {
    pub fn is_simple(&self) -> bool {
        match self {
            MacroCondition::Simple(_) => true,
            _ => false,
        }
    }

    pub fn is_disjunction(&self) -> bool {
        match self {
            MacroCondition::Disjunction { .. } => true,
            _ => false,
        }
    }

    pub fn is_conjunction(&self) -> bool {
        match self {
            MacroCondition::Conjunction { .. } => true,
            _ => false,
        }
    }

    /// Check if the condition is valid. Valid conditions are either Simple,
    /// all disjunction, or all conjunction. I.E. a condition cannot contain
    /// a mixture of disjunction and conjunction.
    pub fn is_valid(&self) -> bool {
        match self {
            MacroCondition::Simple(_) => true,
            MacroCondition::Disjunction { rhs, .. } => rhs.is_disjunction_recursive(),
            MacroCondition::Conjunction { rhs, .. } => rhs.is_conjunction_recursive(),
        }
    }

    /// Recursively check if all conditions are Disjunction or Simple.
    /// Returns `false` if any node is Conjunction.
    pub fn is_disjunction_recursive(&self) -> bool {
        match self {
            MacroCondition::Simple(_) => true,
            MacroCondition::Disjunction { rhs, .. } => rhs.is_disjunction_recursive(),
            MacroCondition::Conjunction { .. } => false,
        }
    }

    /// Recursively check if all conditions are Conjunction or Simple.
    /// Returns `false` if any node is Disjunction.
    pub fn is_conjunction_recursive(&self) -> bool {
        match self {
            MacroCondition::Simple(_) => true,
            MacroCondition::Disjunction { .. } => false,
            MacroCondition::Conjunction { rhs, .. } => rhs.is_conjunction_recursive(),
        }
    }
}

impl TreeNode for MacroCondition {
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

impl ToString for MacroCondition {
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
pub enum IfDefBranch {
    ElseIfDef {
        line: u32,
        line_end_index: u32,
        macro_range: TokenRange,
        indent: TreeStr,
        condition: MacroCondition,
        body: Lines,
        branch: Box<IfDefBranch>,
    },
    Else {
        line: u32,
        line_end_index: u32,
        macro_range: TokenRange,
        indent: TreeStr,
        body: Lines,
        endif_line: u32,
        endif_line_end_index: u32,
        endif_macro_range: TokenRange,
        endif_indent: TreeStr,
    },
    EndIf {
        line: u32,
        line_end_index: u32,
        macro_range: TokenRange,
        indent: TreeStr,
    },
}

impl IfDefBranch {
    /// Get start line of the branch.
    pub fn get_start_line(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef { line, .. } => *line,
            IfDefBranch::Else { line, .. } => *line,
            IfDefBranch::EndIf { line, .. } => *line,
        }
    }

    /// Get end line of this branch.
    pub fn get_end_line(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef { branch, .. } => branch.get_start_line() - 1,
            IfDefBranch::Else { endif_line, .. } => *endif_line - 1,
            // For Endif, the start line and the end line are always the same.
            IfDefBranch::EndIf { line, .. } => *line,
        }
    }
}

#[cfg(feature = "lsp-types")]
impl TextFormatter for IfDefBranch {
    /// Create TextEdits for the macros. This should only manipulate the
    /// whitespace in the line.
    fn format(&self, format_options: &FormatOptions, level: u32) -> Vec<lsp_types::TextEdit> {
        use std::ops::Deref;

        let mut lines = Vec::new();

        let new_indent = if format_options.insert_spaces {
            format!("{: ^1$}", "", (level * format_options.tab_size) as usize)
        } else {
            format!("{:\t^1$}", "", level as usize)
        };

        let start_index = if format_options.insert_spaces {
            format_options.tab_size * level
        } else {
            level
        };

        let new_text = self.to_string();

        match self {
            IfDefBranch::ElseIfDef {
                line,
                line_end_index,
                macro_range: _,
                indent,
                condition: _,
                body,
                branch,
            } => {
                if new_indent != indent.deref()
                    || start_index != self.get_start_index()
                    || (start_index + new_text.len() as u32) != *line_end_index
                {
                    let new_text = new_indent.clone() + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(new_text, *line, 0, *line_end_index));
                }

                lines.extend(
                    body.iter()
                        .flat_map(|line| line.format(format_options, level + 1)),
                );

                lines.extend(branch.format(format_options, level));
            }
            IfDefBranch::Else {
                line,
                line_end_index,
                macro_range: _,
                indent,
                body,
                endif_line,
                endif_line_end_index,
                endif_macro_range,
                endif_indent,
            } => {
                if new_indent != indent.deref()
                    || start_index != self.get_start_index()
                    || (start_index + new_text.len() as u32) != *line_end_index
                {
                    let new_text = new_indent.clone() + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(new_text, *line, 0, *line_end_index));
                }

                lines.extend(
                    body.iter()
                        .flat_map(|line| line.format(format_options, level + 1)),
                );

                let new_text = ENDIF_STR.to_string();
                if new_indent != endif_indent.deref()
                    || start_index != endif_macro_range.start
                    || (start_index + new_text.len() as u32) != *endif_line_end_index
                {
                    let new_text = new_indent + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(
                        new_text,
                        *endif_line,
                        0,
                        *endif_line_end_index,
                    ));
                }
            }
            IfDefBranch::EndIf {
                line,
                line_end_index,
                macro_range: _,
                indent,
            } => {
                if new_indent != indent.deref()
                    || start_index != self.get_start_index()
                    || (start_index + new_text.len() as u32) != *line_end_index
                {
                    let new_text = new_indent + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(new_text, *line, 0, *line_end_index));
                }
            }
        }
        return lines;
    }
}

impl TreeNode for IfDefBranch {
    fn get_start_index(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef { macro_range, .. } => macro_range.start,
            IfDefBranch::Else { macro_range, .. } => macro_range.start,
            IfDefBranch::EndIf { macro_range, .. } => macro_range.start,
        }
    }

    fn get_end_index(&self) -> u32 {
        match self {
            IfDefBranch::ElseIfDef { condition, .. } => condition.get_end_index(),
            IfDefBranch::Else { macro_range, .. } => macro_range.end,
            IfDefBranch::EndIf { macro_range, .. } => macro_range.end,
        }
    }
}

impl ToString for IfDefBranch {
    fn to_string(&self) -> String {
        match self {
            IfDefBranch::ElseIfDef { condition, .. } => {
                format!("{ELSEIFDEF_STR} {}", condition.to_string())
            }
            IfDefBranch::Else { .. } => ELSE_STR.to_string(),
            IfDefBranch::EndIf { .. } => ENDIF_STR.to_string(),
        }
    }
}

vec_wrapper!(IfNotDefClauses, VariableStrings);

#[derive(Debug)]
pub enum IfNotDefBranch {
    Else {
        line: u32,
        line_end_index: u32,
        macro_range: TokenRange,
        indent: TreeStr,
        body: Lines,
        endif_line: u32,
        endif_line_end_index: u32,
        endif_macro_range: TokenRange,
        endif_indent: TreeStr,
    },
    EndIf {
        line: u32,
        line_end_index: u32,
        macro_range: TokenRange,
        indent: TreeStr,
    },
}

impl IfNotDefBranch {
    /// Get the start line of this branch
    pub fn get_start_line(&self) -> u32 {
        match self {
            IfNotDefBranch::Else { line, .. } => *line,
            IfNotDefBranch::EndIf { line, .. } => *line,
        }
    }

    /// Get the end line of this branch.
    pub fn get_end_line(&self) -> u32 {
        match self {
            IfNotDefBranch::Else { endif_line, .. } => *endif_line - 1,
            // For EndIf, the start and end lines are always the same.
            IfNotDefBranch::EndIf { line, .. } => *line,
        }
    }
}

#[cfg(feature = "lsp-types")]
impl TextFormatter for IfNotDefBranch {
    /// Create TextEdits for the macros. This should only manipulate the
    /// whitespace in the line.
    fn format(&self, format_options: &FormatOptions, level: u32) -> Vec<lsp_types::TextEdit> {
        use std::ops::Deref;

        let mut lines = Vec::new();

        let new_indent = if format_options.insert_spaces {
            format!("{: ^1$}", "", (level * format_options.tab_size) as usize)
        } else {
            format!("{:\t^1$}", "", level as usize)
        };

        let start_index = if format_options.insert_spaces {
            format_options.tab_size * level
        } else {
            level
        };

        let new_text = self.to_string();

        match self {
            IfNotDefBranch::Else {
                line,
                line_end_index,
                macro_range: _,
                indent,
                body,
                endif_line,
                endif_line_end_index,
                endif_macro_range,
                endif_indent,
            } => {
                if new_indent != indent.deref()
                    || start_index != self.get_start_index()
                    || (start_index + new_text.len() as u32) != *line_end_index
                {
                    let new_text = new_indent.clone() + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(new_text, *line, 0, *line_end_index));
                }

                lines.extend(
                    body.iter()
                        .flat_map(|line| line.format(format_options, level + 1)),
                );

                let new_text = ENDIF_STR.to_string();
                if new_indent != endif_indent.deref()
                    || start_index != endif_macro_range.start
                    || (start_index + new_text.len() as u32) != *endif_line_end_index
                {
                    let new_text = new_indent + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(
                        new_text,
                        *endif_line,
                        0,
                        *endif_line_end_index,
                    ));
                }
            }
            IfNotDefBranch::EndIf {
                line,
                line_end_index,
                macro_range: _,
                indent,
            } => {
                if new_indent != indent.deref()
                    || start_index != self.get_start_index()
                    || (start_index + new_text.len() as u32) != *line_end_index
                {
                    let new_text = new_indent + new_text.as_str();
                    tracing::info!("Formatting line({line}): '{new_text}'");
                    lines.push(create_text_edit(new_text, *line, 0, *line_end_index));
                }
            }
        }
        return lines;
    }
}

impl TreeNode for IfNotDefBranch {
    fn get_start_index(&self) -> u32 {
        match self {
            IfNotDefBranch::Else { macro_range, .. } => macro_range.start,
            IfNotDefBranch::EndIf { macro_range, .. } => macro_range.start,
        }
    }

    fn get_end_index(&self) -> u32 {
        match self {
            IfNotDefBranch::Else { macro_range, .. } => macro_range.end,
            IfNotDefBranch::EndIf { macro_range, .. } => macro_range.end,
        }
    }
}

impl ToString for IfNotDefBranch {
    fn to_string(&self) -> String {
        match self {
            IfNotDefBranch::Else { .. } => ELSE_STR.to_string(),
            IfNotDefBranch::EndIf { .. } => ENDIF_STR.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum Line {
    /// NOTE: Comments are not really supported by NSPlug. We have them here
    /// because they might be soon.
    Comment {
        comment: Comment,
        line: u32,
        line_end_index: u32,
    },
    Macro {
        macro_type: MacroType,
        comment: Option<Comment>,
        line: u32,
        line_end_index: u32,
        indent: TreeStr,
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
                indent: _,
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
    /// Get the start index in the line for the value
    #[inline]
    fn get_start_index(&self) -> u32 {
        match self {
            Line::Comment {
                comment,
                line: _,
                line_end_index: _,
            } => comment.get_start_index(),
            Line::Macro {
                macro_type,
                comment: _,
                line: _,
                line_end_index: _,
                indent: _,
            } => macro_type.get_start_index(),
            Line::Variable { variable, line: _ } => variable.get_start_index(),
            Line::Error {
                start_line: _,
                end_line: _,
            } => 0,
            Line::EndOfLine { line: _, index } => *index,
        }
    }

    /// Get the end index in the line for the value
    #[inline]
    fn get_end_index(&self) -> u32 {
        match self {
            Line::Comment {
                comment,
                line: _,
                line_end_index: _,
            } => comment.get_end_index(),
            Line::Macro {
                macro_type,
                comment: _,
                line: _,
                line_end_index: _,
                indent: _,
            } => macro_type.get_end_index(),
            Line::Variable { variable, line: _ } => variable.get_end_index(),
            Line::Error {
                start_line: _,
                end_line: _,
            } => 0,
            Line::EndOfLine { line: _, index } => *index,
        }
    }
}

#[cfg(feature = "lsp-types")]
impl TextFormatter for Line {
    fn format(&self, format_options: &FormatOptions, level: u32) -> Vec<lsp_types::TextEdit> {
        match self {
            Line::Macro {
                macro_type,
                comment: _,
                line,
                line_end_index,
                indent,
            } => {
                return macro_type.format(*line, *line_end_index, indent, format_options, level);
            }
            _ => return Vec::new(),
        }
    }
}

impl ToString for Line {
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
                indent: _,
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
