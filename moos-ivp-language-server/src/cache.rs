use crate::parsers::nsplug;
use lsp_types::{Diagnostic, SemanticToken, SemanticTokenModifier, SemanticTokens, Url};
use moos_parser::lexers::TokenMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, trace, warn};

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
    &SemanticTokenModifier::DECLARATION,
    &SemanticTokenModifier::DOCUMENTATION,
    &SemanticTokenModifier::DEPRECATED,
];

enum TokenModifiers {
    /// When no modifiers are needed
    None = 0,
    /// For declarations of symbols.
    Declaration = 0x01,
    /// For occurrences of symbols in documentation.
    Documentation = 0x02,
    /// For symbols that should no longer be used.
    Deprecated = 0x04,
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
enum FileType {
    MoosMission,
    Behavior,
    Plug,
    Script,
    Manifest,
    #[default]
    Other,
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
    pub documents: HashMap<Url, Box<Document>>,
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
    pub fn insert(&mut self, uri: &Url, text: &str) -> &Document {
        let document = self
            .documents
            .entry(uri.clone())
            .or_insert(Box::new(Document::new(uri.clone(), String::new())));

        // Parsers don't handle EOF without an extra new-line. Hack solution is
        // to add a new line to the end of the document.
        let mut text = text.to_string() + "\n";
        document.text = text;
        document.refresh();
        return document;
    }
}

#[derive(Debug)]
pub struct Document {
    pub uri: Url,
    pub text: String,
    pub file_type: FileType,
    pub semantic_tokens: TokenMap<SemanticTokenInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Document {
    pub fn new(uri: Url, text: String) -> Self {
        Self {
            uri,
            text,
            file_type: FileType::Other,
            semantic_tokens: TokenMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.clear();
        nsplug::parse(self);
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

    pub fn clear(&mut self) {
        self.semantic_tokens.clear();
        self.diagnostics.clear();
    }
}

struct Delcaration {}

struct Definition {}

struct CacheData {
    document: Url,
    text: String,
}

type Cache = HashMap<Url, CacheData>;
