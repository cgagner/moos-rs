use std::{ops::Deref, sync::Arc};

use super::new_error_diagnostic;
use crate::cache::{Document, SemanticTokenInfo, TokenTypes};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentLink, FoldingRange, InlayHint, InlayHintKind,
    InlayHintLabel, Position,
};
use moos_parser::{
    base::TreeNode,
    behavior::{
        self,
        error::{BehaviorParseError, BehaviorParseErrorKind},
        lexer::{State, Token},
        tree::{
            Assignment, BehaviorBlock, Comment, Line, Lines, Quote, Values, Variable,
            VariableStrings,
        },
    },
    lexers::{self, Location, TokenMap, TokenRange},
    BehaviorParser, ParseError,
};
use tracing::{debug, error, info, trace, warn};

pub fn parse(document: &mut Document) {
    // NOTE: This clone is of a Arc<str>. It should not perform a deep copy
    // of the underlying str. This is needed to be able to pass a mutable
    // reference to the Document and the Result from the parser to other
    // functions.
    let text = Arc::clone(&document.text);
    let lexer = moos_parser::BehaviorLexer::new(text.deref());
    let mut state = moos_parser::behavior::lexer::State::default();
    let result = BehaviorParser::new().parse(&mut state, text.deref(), lexer);

    info!("Parse Results: {result:?}");

    if let Ok(lines) = result {
        handle_lines(document, &lines);
    }

    state.errors.iter().for_each(|e| {
        error!("Parse Error: {e:?}");
    });
    // TODO: Add new method to handle converting errors into diagnostics
    // TODO: Only create diagnostics if the client supports diagnostics
    // TODO: Need to handle dropped tokens
    for e in state.errors {
        match e.error {
            ParseError::User { error } => match error.kind {
                BehaviorParseErrorKind::UnexpectedAssignment => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        String::from("Unexpected assignment."),
                    );
                    document.diagnostics.push(d);
                }
                BehaviorParseErrorKind::MissingNewLine => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        String::from("Missing new line after application name."),
                    );
                    document.diagnostics.push(d);
                }
                BehaviorParseErrorKind::MissingTrailing(c) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Missing trailing character {c:?}"),
                    );
                    document.diagnostics.push(d);
                }
                BehaviorParseErrorKind::UnexpectedSymbol(c) => {}
                BehaviorParseErrorKind::UnexpectedComment(comment) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unexpected comment: {comment}"),
                    );
                    document.diagnostics.push(d);
                }
                BehaviorParseErrorKind::UnknownMacro(text) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unknown macro: {text}"),
                    );
                    document.diagnostics.push(d);
                }
                BehaviorParseErrorKind::MissingEndIf => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Missing #endif"),
                    );
                    document.diagnostics.push(d);
                }
            },
            ParseError::UnrecognizedToken { token, expected } => {
                let (loc_start, token, loc_end) = token;
                let d = new_error_diagnostic(
                    &loc_start,
                    &loc_end,
                    format!("Unrecognized token: {:?}. Expected: {:#?}", token, expected),
                );
                document.diagnostics.push(d);
            }
            ParseError::UnrecognizedEOF { location, expected } => {
                let d = new_error_diagnostic(
                    &location,
                    &location,
                    format!(
                        "Unrecognized end of file at {:?}. Expected: {:#?}",
                        location, expected
                    ),
                );
                document.diagnostics.push(d);
            }
            _ => {}
        }
    }
}

fn handle_lines(document: &mut Document, lines: &Lines) {
    use moos_parser::behavior::tree::Line::*;
    for l in lines {
        match l {
            Comment { comment, line } => {
                handle_comment(document, *line, &comment);
            }
            Variable { variable, line } => {
                // TODO: Add a reference to the variable

                handle_variable(document, *line, &variable);
            }
            Assignment { assignment, line } => {
                // TODO: Add a lookup to get the token type and modifier for the keywords
                handle_assignment(document, *line, &assignment, TokenTypes::Macro, 0)
            }
            BehaviorBlock {
                behavior_block,
                line,
                range,
            } => {
                // NOTE: A BehaviorBlock inside another BehaviorBlock is handled
                // by the parser.
                handle_behavior_block(document, *line, range, behavior_block);
            }
            Initialize {
                assignment,
                deferred: _,
                line,
                range,
            } => {
                handle_keyword(document, *line, range);
                handle_assignment(document, *line, assignment, TokenTypes::Variable, 0);
            }
            _ => {}
        }
    }
}

fn handle_keyword(document: &mut Document, line: u32, range: &TokenRange) {
    document.semantic_tokens.insert(
        line,
        range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Keyword as u32,
            token_modifiers: 0,
        },
    );
}

fn handle_assignment(
    document: &mut Document,
    line: u32,
    assignment: &Assignment,
    string_type: TokenTypes,
    string_modifiers: u32,
) {
    handle_variable_strings(
        document,
        line,
        &assignment.name,
        string_type,
        string_modifiers,
    );

    handle_values(document, line, &assignment.value);

    if let Some(comment) = &assignment.comment {
        handle_comment(document, line, &comment);
    }
}

fn handle_quote(document: &mut Document, line: u32, quote: &Quote) {
    // Insert all of the variables first so they take priority
    quote.content.iter().for_each(|v| match v {
        behavior::tree::Value::Variable(variable) => handle_variable(document, line, variable),
        _ => {}
    });

    // Then insert one large string for the span of the quote
    document.semantic_tokens.insert(
        line,
        quote.range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::String as u32,
            token_modifiers: 0,
        },
    );
}

fn handle_variable_strings(
    document: &mut Document,
    line: u32,
    values: &VariableStrings,
    string_type: TokenTypes,
    string_modifiers: u32,
) {
    for v in values {
        match v {
            behavior::tree::VariableString::String(text, range) => {
                handle_string(document, line, text, range, string_type, string_modifiers);
            }
            behavior::tree::VariableString::Variable(variable) => {
                handle_variable(document, line, variable);
            }
        }
    }
}

fn handle_string(
    document: &mut Document,
    line: u32,
    _text: &str,
    range: &TokenRange,
    string_type: TokenTypes,
    string_modifiers: u32,
) {
    document.semantic_tokens.insert(
        line,
        range.clone(),
        SemanticTokenInfo {
            token_type: string_type as u32,
            token_modifiers: string_modifiers,
        },
    );
}

fn handle_variable(document: &mut Document, line: u32, variable: &Variable) {
    match variable {
        behavior::tree::Variable::Regular { text: _, range }
        | behavior::tree::Variable::Partial { text: _, range } => {
            document.semantic_tokens.insert(
                line,
                range.clone(),
                SemanticTokenInfo {
                    token_type: TokenTypes::Variable as u32,
                    token_modifiers: 0,
                },
            );
        }
    }
}

fn handle_comment(document: &mut Document, line: u32, comment: &Comment) {
    document.semantic_tokens.insert(
        line,
        comment.range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Comment as u32,
            token_modifiers: 0,
        },
    );
}

fn handle_values(document: &mut Document, line: u32, values: &Values) {
    for v in values {
        match v {
            behavior::tree::Value::Boolean(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Type as u32,
                        token_modifiers: 0,
                    },
                );
            }
            behavior::tree::Value::Integer(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            behavior::tree::Value::Float(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            behavior::tree::Value::String(_text, _range) => {
                // TODO: Should we color these differently than a quoted string?
                // For now, we'll leave these unformatted
            }
            behavior::tree::Value::Quote(quote) => handle_quote(document, line, quote),
            behavior::tree::Value::Variable(variable) => handle_variable(document, line, variable),
        }
    }
}

fn handle_behavior_block(
    document: &mut Document,
    line: u32,
    range: &TokenRange,
    behavior_block: &BehaviorBlock,
) {
    handle_keyword(document, line, range);
    // TODO: Should check if the behavior name is in the path and display a
    // warning if it is not.
    handle_variable_strings(
        document,
        line,
        &behavior_block.behavior_name,
        TokenTypes::Struct,
        0,
    );

    if let Some(comment) = &behavior_block.behavior_block_comment {
        handle_comment(document, line, &comment);
    }

    // Prelude Comments
    if !behavior_block.prelude_comments.is_empty() {
        // NOTE: Invalid lines are handled by the parser. This should just
        // add comments.
        handle_lines(document, &behavior_block.prelude_comments);
    }

    // Open Curly Comment
    if let Some(comment) = &behavior_block.open_curly_comment {
        handle_comment(document, behavior_block.open_curly_line, &comment);
    }

    // TODO: Add warning if BehaviorBlock contains a initialize

    // Add folding range for BehaviorBlock block
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = line;
    folding_range.end_line = behavior_block.close_curly_line;
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    // Handle Body.
    // TODO: Add lookup to see if assignments contain keywords

    handle_lines(document, &behavior_block.body);

    // TODO: This needs to be updated to copy case

    let inlay_text = format!(
        "BehaviorBlock = {}",
        behavior_block.behavior_name.to_string()
    );

    add_inlay_text(
        document,
        behavior_block.close_curly_line,
        behavior_block.close_curly_index + 1,
        inlay_text.as_str(),
    );

    if let Some(comment) = &behavior_block.close_curly_comment {
        handle_comment(document, behavior_block.close_curly_line, &comment);
    }
}

#[inline]
fn add_inlay_text(document: &mut Document, line: u32, index: u32, text: &str) {
    document.inlay_hints.push(InlayHint {
        position: Position {
            line: line,
            character: index,
        },
        label: InlayHintLabel::String(text.to_string()),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: Some(true),
        data: None,
    });
}
