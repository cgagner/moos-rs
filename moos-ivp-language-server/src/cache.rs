use crate::parsers::{behavior, moos, nsplug};

use lsp_types::{
    CompletionContext, CompletionItem, CompletionList, CompletionResponse, Diagnostic,
    DocumentLink, FoldingRange, FormattingOptions, InlayHint, SemanticToken, SemanticTokenModifier,
    SemanticTokens, TextEdit, Url,
};
use moos_parser::{lexers::TokenMap, nsplug::tree::ENDIF_STR};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
pub struct SemanticTokenInfo {
    pub token_type: u32,
    pub token_modifiers: u32,
}

/*
 * namespace	For identifiers that declare or reference a namespace, module, or package.
 * class	For identifiers that declare or reference a class type.
 * enum	For identifiers that declare or reference an enumeration type.
 * interface	For identifiers that declare or reference an interface type.
 * struct	For identifiers that declare or reference a struct type.
 * typeParameter	For identifiers that declare or reference a type parameter.
 * type	For identifiers that declare or reference a type that is not covered above.
 * parameter	For identifiers that declare or reference a function or method parameters.
 * variable	For identifiers that declare or reference a local or global variable.
 * property	For identifiers that declare or reference a member property, member field, or member variable.
 * enumMember	For identifiers that declare or reference an enumeration property, constant, or member.
 * decorator	For identifiers that declare or reference decorators and annotations.
 * event	For identifiers that declare an event property.
 * function	For identifiers that declare a function.
 * method	For identifiers that declare a member function or method.
 * macro	For identifiers that declare a macro.
 * label	For identifiers that declare a label.
 * comment	For tokens that represent a comment.
 * string	For tokens that represent a string literal.
 * keyword	For tokens that represent a language keyword.
 * number	For tokens that represent a number literal.
 * regexp	For tokens that represent a regular expression literal.
 * operator	For tokens that represent an operator.
 */

#[derive(Debug, Copy, Clone)]
pub enum TokenTypes {
    /// For tokens that represent a comment.
    Comment = 0,
    /// For tokens that represent a language keyword.
    Keyword,
    /// For identifiers that declare or reference a local or global variable.
    Variable,
    /// For tokens that represent a string literal.
    String,
    /// For tokens that represent a number literal.
    Number,
    /// For identifiers that declare a macro.
    Macro,
    /// For tokens that represent an operator
    Operator,
    /// For types
    Type,
    /// Namespace
    Namespace,
    /// Struct
    Struct,
}

impl Into<u32> for TokenTypes {
    fn into(self) -> u32 {
        self as u32
    }
}

/*
 * declaration	For declarations of symbols.
 * definition	For definitions of symbols, for example, in header files.
 * readonly	For readonly variables and member fields (constants).
 * static	For class members (static members).
 * deprecated	For symbols that should no longer be used.
 * abstract	For types and member functions that are abstract.
 * async	For functions that are marked async.
 * modification	For variable references where the variable is assigned to.
 * documentation	For occurrences of symbols in documentation.
 * defaultLibrary	For symbols that are part of the standard library.
 */

pub(crate) const TOKEN_MODIFIERS: &'static [&'static SemanticTokenModifier] = &[
    &SemanticTokenModifier::DEPRECATED,
    &SemanticTokenModifier::DECLARATION,
    &SemanticTokenModifier::DOCUMENTATION,
];

pub enum TokenModifiers {
    /// When no modifiers are needed
    None = 0,
    /// For symbols that should no longer be used.
    Deprecated = 0x01,
    /// For declarations of symbols.
    Declaration = 0x02,
    /// For occurrences of symbols in documentation.
    Documentation = 0x04,
}

impl Into<u32> for TokenModifiers {
    fn into(self) -> u32 {
        self as u32
    }
}

impl core::ops::BitOr for TokenModifiers {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u32 | rhs as u32
    }
}

impl core::ops::BitOr<TokenModifiers> for u32 {
    type Output = u32;

    fn bitor(self, rhs: TokenModifiers) -> Self::Output {
        self | rhs as u32
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum FileType {
    MoosMission,
    PlugMoosMission,
    Behavior,
    PlugBehavior,
    Plug,
    Script,
    Manifest,
    #[default]
    Other,
}

impl FileType {
    pub fn from_uri(uri: &Url) -> Self {
        if let Ok(path) = uri.to_file_path() {
            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");
            return Self::from_filename(filename);
        } else {
            return Self::default();
        }
    }

    pub fn from_filename(filename: &str) -> Self {
        let filename = filename.to_ascii_lowercase();

        let extension = std::path::Path::new(filename.as_str())
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        if filename.starts_with("plug_") || filename.starts_with("meta_") {
            match extension {
                "moos" | "moos++" => return Self::PlugMoosMission,
                "bhv" | "bhv++" => return Self::PlugBehavior,
                _ => return Self::Plug,
            }
        }

        if filename.starts_with("app_")
            || filename.starts_with("moos_")
            || filename.starts_with("data_")
        {
            match extension {
                "plug" | "def" => return Self::PlugMoosMission,
                _ => {}
            }
        }

        if filename.starts_with("bhv_") {
            match extension {
                "plug" | "def" => return Self::PlugBehavior,
                _ => {}
            }
        }

        match extension {
            "moos" | "moos++" => return Self::MoosMission,
            "bhv" | "bhv++" => return Self::Behavior,
            "plug" | "def" => return Self::Plug,
            "bash" | "sh" | "zsh" => return Self::Script,
            "mfs" | "gfs" => return Self::Manifest,
            _ => return Self::Other,
        }
    }

    pub fn is_plug(&self) -> bool {
        match self {
            Self::Plug | Self::PlugBehavior | Self::PlugMoosMission => true,
            _ => false,
        }
    }

    pub fn is_moos_mission(&self) -> bool {
        match self {
            Self::MoosMission | Self::PlugMoosMission => true,
            _ => false,
        }
    }

    pub fn is_behavior(&self) -> bool {
        match self {
            Self::PlugBehavior | Self::Behavior => true,
            _ => false,
        }
    }
}

enum ReferenceType {
    PlugVariable,
    AntlerProcessConfig,
}

enum DocumentLinkType {
    NsplugInclude,
    IvpBehavior,
}

pub struct Project {
    pub root: String,
    pub documents: HashMap<Url, Document>,
}

impl Project {
    pub fn new(root: String) -> Self {
        Self {
            root,
            documents: HashMap::new(),
        }
    }

    /// Creates or updates a `Document` with the specified `uri` and `text`
    /// and updates the cache for the document.
    ///
    /// NOTE: This method takes ownership of the `text`
    pub fn insert(&mut self, uri: &Url, text: String) -> &Document {
        let document = self
            .documents
            .entry(uri.clone())
            .or_insert(Document::new(uri.clone(), String::new()));

        document.text = text.into();
        document.refresh();
        return document;
    }
}

#[derive(Debug)]
pub struct Document {
    pub uri: Url,
    pub text: Arc<str>,
    pub file_type: FileType,
    pub semantic_tokens: TokenMap<SemanticTokenInfo>,
    pub diagnostics: Vec<Diagnostic>,
    pub folding_ranges: Vec<FoldingRange>,
    pub document_links: Vec<DocumentLink>,
    pub inlay_hints: Vec<InlayHint>,
    pub plug_lines: moos_parser::nsplug::tree::Lines,
}

impl Document {
    pub fn new(uri: Url, text: String) -> Self {
        let file_type = FileType::from_uri(&uri);
        Self {
            uri,
            text: text.into(),
            file_type,
            semantic_tokens: TokenMap::new(),
            diagnostics: Vec::new(),
            folding_ranges: Vec::new(),
            document_links: Vec::new(),
            inlay_hints: Vec::new(),
            plug_lines: moos_parser::nsplug::tree::Lines::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.clear();

        // Always parser the nsplug first
        match self.file_type {
            FileType::PlugMoosMission | FileType::PlugBehavior | FileType::Plug => {
                nsplug::parse(self)
            }
            _ => {}
        }

        match self.file_type {
            FileType::MoosMission | FileType::PlugMoosMission => moos::parse(self),
            FileType::Behavior | FileType::PlugBehavior => behavior::parse(self),
            FileType::Plug => {}
            FileType::Script => {}
            FileType::Manifest => {}
            FileType::Other => {}
        }
    }

    pub fn clear(&mut self) {
        self.semantic_tokens.clear();
        self.diagnostics.clear();
        self.folding_ranges.clear();
        self.document_links.clear();
        self.inlay_hints.clear();
    }

    pub fn get_semantic_tokens(&self) -> SemanticTokens {
        let mut tokens = SemanticTokens::default();

        self.semantic_tokens.relative_iter().for_each(|token| {
            tokens.data.push(SemanticToken {
                delta_line: token.delta_line,
                delta_start: token.delta_start,
                length: token.length,
                token_type: token.token.token_type,
                token_modifiers_bitset: token.token.token_modifiers,
            });
        });

        return tokens;
    }

    pub fn add_folding_range(&mut self, folding_range: FoldingRange) -> bool {
        // Folding range end must be after the start range.
        if folding_range.end_line <= folding_range.start_line {
            return false;
        }

        // Do not add folding ranges that are less than two lines.
        if folding_range.end_line - folding_range.start_line < 2 {
            return false;
        }

        // Checks if r1 is inside of r2 - Assumes a check for entirely before
        // and entirely after has already been completed.
        let is_inside = |r1: &FoldingRange, r2: &FoldingRange| -> bool {
            r1.start_line > r2.start_line
                && r1.start_line < r2.end_line
                && r1.end_line < r2.end_line
        };

        // Check for overlaps.. It is fine if a folding range is entirely
        // inside of another range.
        for existing_range in &self.folding_ranges {
            if folding_range.end_line < existing_range.start_line
                || folding_range.start_line > existing_range.end_line
                || is_inside(&folding_range, &existing_range)
                || is_inside(&existing_range, &folding_range)
            {
                continue;
            } else {
                return false;
            }
        }

        // If we get this far, then it is a valid range
        self.folding_ranges.push(folding_range);
        return true;
    }

    pub fn get_formats(&self, options: &FormattingOptions) -> Option<Vec<TextEdit>> {
        let format_options = options.into();

        tracing::info!("Get Formats");

        // Always parser the nsplug first
        match self.file_type {
            FileType::PlugMoosMission | FileType::PlugBehavior | FileType::Plug => {
                return nsplug::format(self, &format_options)
            }
            _ => {}
        }

        // TODO: Handle other file types
        match self.file_type {
            FileType::MoosMission | FileType::PlugMoosMission => {}
            FileType::Behavior | FileType::PlugBehavior => {
                // TODO: Add Behavior parser
            }
            FileType::Plug => {}
            FileType::Script => {}
            FileType::Manifest => {}
            FileType::Other => {}
        }

        None
    }

    pub fn get_completion(
        &self,
        position: lsp_types::Position,
        context: Option<CompletionContext>,
    ) -> Option<CompletionResponse> {
        // TODO: Need to add completion for other file types
        // TODO: Should moved each file type into a different module so this
        //       method does not end up being 1000 lines.
        // TODO: NSPlug
        //       - Add completion for include paths
        //       - Add else and else if when inside an ifdef or ifndef block.

        if !self.file_type.is_plug() {
            return None;
        }

        let line_text = self
            .text
            .lines()
            .nth(position.line as usize)
            .unwrap_or_default();

        if let Some(context) = context {
            if let Some(trigger) = context.trigger_character {
                if trigger == "#" {
                    if line_text.trim() != "#" {
                        return None;
                    }

                    let indent = if let Some((indent, _)) = line_text.split_once("#") {
                        indent
                    } else {
                        ""
                    };

                    let endif_text = format!("{indent}{ENDIF_STR}\n");

                    let ifdef_completion = CompletionItem {
                        label: "ifdef ".to_string(),
                        detail: Some("NSPlug #ifdef".to_string()),
                        additional_text_edits: Some(vec![TextEdit {
                            range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: position.line + 1,
                                    character: 0,
                                },
                                end: lsp_types::Position {
                                    line: position.line + 1,
                                    character: 0,
                                },
                            },
                            // TODO: Need to add the indent
                            new_text: endif_text.clone(),
                        }]),
                        ..Default::default()
                    };
                    let ifndef_completion = CompletionItem {
                        label: "ifndef ".to_string(),
                        detail: Some("NSPlug #ifndef".to_string()),
                        additional_text_edits: Some(vec![TextEdit {
                            range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: position.line + 1,
                                    character: 0,
                                },
                                end: lsp_types::Position {
                                    line: position.line + 1,
                                    character: 0,
                                },
                            },
                            new_text: endif_text,
                        }]),
                        ..Default::default()
                    };
                    let list = CompletionList {
                        is_incomplete: false,

                        items: vec![
                            ifdef_completion,
                            ifndef_completion,
                            CompletionItem::new_simple(
                                "include ".to_string(),
                                "NSPlug #include".to_string(),
                            ),
                            CompletionItem::new_simple(
                                "define ".to_string(),
                                "NSPlug #define".to_string(),
                            ),
                        ],
                    };

                    return Some(CompletionResponse::List(list));
                } else {
                    tracing::info!("Unknown completion trigger: {trigger}");
                }
            }
        }
        None
    }
}

struct Delcaration {}

struct Definition {}

struct CacheData {
    document: Url,
    text: String,
}

type Cache = HashMap<Url, CacheData>;
