use std::collections::HashMap;

use lsp_types::{
    Diagnostic, DiagnosticSeverity, SemanticToken, SemanticTokenModifier, SemanticTokens, Url,
};
use moos_parser::{
    lexers::{self, Location, TokenMap},
    nsplug::{
        self,
        error::{PlugParseError, PlugParseErrorKind},
        lexer::{State, Token, TokenListener},
    },
    Lexer, LinesParser, ParseError, PlugParser,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
struct SemanticTokenInfo {
    token_type: u32,
    token_modifiers: u32,
}

enum TokenTypes {
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
    uri: Url,
    text: String,
    file_type: FileType,
    token_collector: TokenCollector,
    pub diagnostics: Vec<Diagnostic>,
}

impl Document {
    pub fn new(uri: Url, text: String) -> Self {
        Self {
            uri,
            text,
            file_type: FileType::Other,
            token_collector: TokenCollector::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.clear();

        info!("Parsing: {:?}", &self.text);

        let mut lexer = moos_parser::nsplug::lexer::Lexer::new(&self.text);
        lexer.add_listener(&mut self.token_collector);
        let mut state = moos_parser::nsplug::lexer::State::default();
        let result = PlugParser::new().parse(&mut state, &self.text, lexer);

        let iter = self.diagnostics.iter();
        // TODO: Add new method to handle converting errors into diagnostics
        // TODO: Only create diagnostics if the client supports diagnostics
        for e in state.errors {
            match e.error {
                ParseError::User { error } => match error.kind {
                    PlugParseErrorKind::InvalidConfigBlock => {}
                    PlugParseErrorKind::MissingNewLine => {
                        let d = Diagnostic::new(
                            lsp_types::Range {
                                start: lsp_types::Position {
                                    line: error.loc_start.line,
                                    character: error.loc_start.index,
                                },
                                end: lsp_types::Position {
                                    line: error.loc_end.line,
                                    character: error.loc_end.index,
                                },
                            },
                            Some(DiagnosticSeverity::ERROR),
                            None,
                            None,
                            String::from("Missing new line after application name."),
                            None,
                            None,
                        );
                        self.diagnostics.push(d);
                    }
                    PlugParseErrorKind::MissingTrailing(c) => {}
                    PlugParseErrorKind::UnexpectedSymbol(c) => {}
                    _ => {}
                },
                ParseError::UnrecognizedToken { token, expected } => {
                    let (loc_start, token, loc_end) = token;
                    let d = Diagnostic::new(
                        lsp_types::Range {
                            start: lsp_types::Position {
                                line: loc_start.line,
                                character: loc_start.index,
                            },
                            end: lsp_types::Position {
                                line: loc_end.line,
                                character: loc_end.index,
                            },
                        },
                        Some(DiagnosticSeverity::ERROR),
                        None,
                        None,
                        format!("Unrecognized token: {:?}. Expected: {:?}", token, expected),
                        None,
                        None,
                    );
                    self.diagnostics.push(d);
                }
                _ => {}
            }
        }
    }

    pub fn get_semantic_tokens(&self) -> SemanticTokens {
        let mut tokens = SemanticTokens::default();

        self.token_collector
            .semantic_tokens
            .relative_iter()
            .for_each(|token| {
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
        self.token_collector.semantic_tokens.clear();
        self.diagnostics.clear();
    }
}

#[derive(Debug, Default)]
struct TokenCollector {
    pub semantic_tokens: TokenMap<SemanticTokenInfo>,
}
impl TokenCollector {
    pub fn new() -> Self {
        Self {
            semantic_tokens: TokenMap::<SemanticTokenInfo>::new(),
        }
    }
}
impl TokenListener for TokenCollector {
    fn handle_token(
        &mut self,
        token: &nsplug::lexer::Token,
        start_loc: &lexers::Location,
        end_loc: &lexers::Location,
    ) {
        // This method seems flawed.
        // PartialQuotes and PartialVariables seems to break

        let token_info = match token {
            Token::Comment(_comment) => Some(SemanticTokenInfo {
                token_type: TokenTypes::Comment as u32,
                token_modifiers: 0,
            }),
            Token::PlugVariable(_name) | Token::PlugUpperVariable(_name) => {
                Some(SemanticTokenInfo {
                    token_type: TokenTypes::Variable as u32,
                    token_modifiers: 0,
                })
            }
            Token::Float(_, _) | Token::Integer(_, _) => Some(SemanticTokenInfo {
                token_type: TokenTypes::Number as u32,
                token_modifiers: 0,
            }),
            Token::MacroDefine
            | Token::MacroElse
            | Token::MacroElseIfDef
            | Token::MacroEndIf
            | Token::MacroIfDef
            | Token::MacroIfNotDef
            | Token::MacroInclude => Some(SemanticTokenInfo {
                token_type: TokenTypes::Macro as u32,
                token_modifiers: 0,
            }),
            Token::Quote(_value) => Some(SemanticTokenInfo {
                token_type: TokenTypes::String as u32,
                token_modifiers: 0,
            }),
            Token::PartialQuote(_value, _c) => Some(SemanticTokenInfo {
                token_type: TokenTypes::String as u32,
                token_modifiers: 0,
            }),
            /*
            Token::BlockKeyword(key) => {
                // TODO:  This should check the value of name for the current
                // application
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Keyword as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::Key(name) => {
                let mut found_keyword = false;

                if let Some(app) = &self.current_app {
                    if self.keywords.contains(name.to_lowercase().as_str()) {
                        found_keyword = true;
                    }
                    // TODO:  This should check the value of name for the current
                    // application
                } else {
                    if self.global_keywords.contains(name.to_lowercase().as_str()) {
                        found_keyword = true;
                    }
                }

                if found_keyword {
                    self.cache.semantic_tokens.push(SemanticToken {
                        delta_line: delta_line,
                        delta_start: delta_index,
                        length: length,
                        token_type: TokenTypes::Variable as u32,
                        token_modifiers_bitset: 0,
                    });

                    added = true;
                }
            }
            Token::EnvVariable(name)
            | Token::PlugVariable((name))
            | Token::PlugUpperVariable(name) => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Variable as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::PartialEnvVariable(value)
            | Token::PartialPlugVariable(value)
            | Token::PartialPlugUpperVariable(value) => {
                let bracket = match token {
                    Token::PartialEnvVariable(_) => '}',
                    Token::PartialPlugVariable(_) | Token::PartialPlugUpperVariable(_) => ')',
                    _ => '}',
                };
                let d = Diagnostic::new(
                    lsp_types::Range {
                        start: lsp_types::Position {
                            line: start_loc.line,
                            character: start_loc.index,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line,
                            character: end_loc.index,
                        },
                    },
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    None,
                    format!("Missing closing bracket for variable: '{}'", bracket).to_owned(),
                    None,
                    None,
                );
                self.cache.diagnostics.push(d);

                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line,
                    delta_start: delta_index,
                    length,
                    token_type: TokenTypes::Variable as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::Quote(value) => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line,
                    delta_start: delta_index,
                    length,
                    token_type: TokenTypes::String as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::PartialQuote(value, c) => {
                let d = Diagnostic::new(
                    lsp_types::Range {
                        start: lsp_types::Position {
                            line: start_loc.line,
                            character: start_loc.index,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line,
                            character: end_loc.index,
                        },
                    },
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    None,
                    format!("Missing closing quote mark: {}", c),
                    None,
                    None,
                );
                self.cache.diagnostics.push(d);

                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line,
                    delta_start: delta_index,
                    length,
                    token_type: TokenTypes::String as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::UnknownMacro(value) => {
                let d = Diagnostic::new(
                    lsp_types::Range {
                        start: lsp_types::Position {
                            line: start_loc.line,
                            character: start_loc.index,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line,
                            character: end_loc.index,
                        },
                    },
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    None,
                    format!("Unknown macro: {}", value),
                    None,
                    None,
                );
                self.cache.diagnostics.push(d);

                // added = true;
            }
            Token::Float(_, _) | Token::Integer(_, _) => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Number as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::Boolean(_, _) => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Keyword as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::DefineKeyword => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Macro as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            Token::OrOperator | Token::AndOperator => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Operator as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
            // TODO: We should differentiate between environment variables and
            // regular variables
            _ => {}
            */
            _ => None,
        };

        if let Some(token_info) = token_info {
            self.semantic_tokens
                .insert(*start_loc, *end_loc, token_info);
        }
    }
}

struct Delcaration {}

struct Definition {}

struct CacheData {
    document: Url,
    text: String,
}

type Cache = HashMap<Url, CacheData>;
