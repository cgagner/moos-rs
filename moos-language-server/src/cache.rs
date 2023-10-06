use core::cmp::max;
use lsp_types::{Diagnostic, DiagnosticSeverity, SemanticToken};
use moos_parser::lexer::{self, Token, TokenListener};
use std::collections::HashSet;

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
pub(crate) const TOKEN_TYPES: &'static [&'static str] = &[
    "comment", "keyword", "variable", "string", "number", "macro", "type",
];

pub(crate) const MOOS_GLOBAL_KEYWORDS: [&'static str; 6] = [
    "community",
    "serverhost",
    "serverport",
    "latorigin",
    "longorigin",
    "moostimewarp",
];
pub(crate) const MOOS_APP_KEYWORDS: [&'static str; 3] = ["CommsTick", "AppTick", "MaxAppTick"];

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

pub(crate) const TOKEN_MODIFIERS: &'static [&'static str] =
    &["declaration", "documentation", "deprecated"];

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

pub(crate) struct Cache {
    pub(crate) semantic_tokens: Vec<SemanticToken>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Cache {
    pub(crate) fn new() -> Self {
        Self {
            semantic_tokens: vec![],
            diagnostics: vec![],
        }
    }
}

pub(crate) struct MoosTokenListener<'c> {
    previous_line: u32,
    previous_index: u32,
    cache: &'c mut Cache,
    global_keywords: HashSet<&'static str>,
    keywords: HashSet<&'static str>,
    current_app: Option<String>,
}

impl<'c> MoosTokenListener<'c> {
    pub(crate) fn new(cache: &'c mut Cache) -> Self {
        Self {
            previous_line: 0,
            previous_index: 0,
            cache,
            global_keywords: HashSet::from(MOOS_GLOBAL_KEYWORDS),
            keywords: HashSet::from(MOOS_APP_KEYWORDS),
            current_app: None,
        }
    }
}

impl<'c> TokenListener for MoosTokenListener<'c> {
    fn handle_token(
        &mut self,
        token: &lexer::Token,
        start_loc: &lexer::Location,
        end_loc: &lexer::Location,
    ) {
        // This method seems flawed.
        // PartialQuotes and PartialVariables seems to break
        let mut added = false;
        let length = max((end_loc.index - start_loc.index) as u32, 0);
        let delta_line = max(start_loc.line as u32 - self.previous_line, 0);
        let delta_index = if delta_line > 0 {
            start_loc.index as u32
        } else {
            max(start_loc.index as u32 - self.previous_index, 0)
        };

        match token {
            Token::Comment(_comment) => {
                self.cache.semantic_tokens.push(SemanticToken {
                    delta_line: delta_line,
                    delta_start: delta_index,
                    length: length,
                    token_type: TokenTypes::Comment as u32,
                    token_modifiers_bitset: 0,
                });
                added = true;
            }
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
                            line: start_loc.line as u32,
                            character: start_loc.index as u32,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line as u32,
                            character: end_loc.index as u32,
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
                            line: start_loc.line as u32,
                            character: start_loc.index as u32,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line as u32,
                            character: end_loc.index as u32,
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
                            line: start_loc.line as u32,
                            character: start_loc.index as u32,
                        },
                        end: lsp_types::Position {
                            line: end_loc.line as u32,
                            character: end_loc.index as u32,
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
            Token::MacroDefine
            | Token::MacroElse
            | Token::MacroElseIfDef
            | Token::MacroEndIf
            | Token::MacroIfDef
            | Token::MacroIfNotDef
            | Token::MacroInclude => {
                log::error!("Found macro {:?}", token);
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
        }
        if added {
            log::debug!(
                "token: {:?}\n  start_index: {}\n  previous_index: {}\n  end_index: {}\n  length: {}",
                token,
                start_loc.index,
                self.previous_index,
                end_loc.index,
                length,
            );

            self.previous_line = start_loc.line as u32;
            self.previous_index = start_loc.index as u32;
        }
    }
}
