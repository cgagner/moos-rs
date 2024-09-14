use std::{ops::Deref, sync::Arc};

use super::{new_error_diagnostic, new_warning_diagnostic};
use crate::cache::{Document, SemanticTokenInfo, TokenModifiers, TokenTypes};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DocumentLink, FoldingRange, InlayHint,
    InlayHintKind, InlayHintLabel, Position,
};
use moos_parser::{
    base::TreeNode,
    behavior::{
        self,
        error::{BehaviorParseError, BehaviorParseErrorKind},
        lexer::{State, Token},
        tree::{
            Assignment, BehaviorBlock, Comment, Line, Lines, Quote, SetBlock, UnknownBlock, Values,
            Variable, VariableStrings,
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
        handle_lines(document, &lines, 0);
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

fn handle_lines(document: &mut Document, lines: &Lines, token_modifiers: u32) {
    use moos_parser::behavior::tree::Line::*;
    for l in lines {
        match l {
            Comment { comment, line } => {
                handle_comment(document, *line, &comment, token_modifiers);
            }
            Variable { variable, line } => {
                // TODO: Add a reference to the variable

                handle_variable(document, *line, &variable, token_modifiers);
            }
            Assignment { assignment, line } => {
                // TODO: Add a lookup to get the token type and modifier for the keywords
                handle_assignment(
                    document,
                    *line,
                    &assignment,
                    TokenTypes::Macro,
                    token_modifiers,
                )
            }
            BehaviorBlock {
                behavior_block,
                line,
                range,
            } => {
                // NOTE: A BehaviorBlock inside another BehaviorBlock is handled
                // by the parser.
                handle_behavior_block(document, *line, range, behavior_block, token_modifiers);
            }
            UnknownBlock {
                unknown_block,
                line,
                range: _,
            } => {
                handle_unknown_block(document, *line, unknown_block, token_modifiers);
            }
            SetBlock {
                set_block,
                line,
                range,
            } => {
                // NOTE: A SetBlock inside another SetBlock is handled
                // by the parser.
                handle_set_block(document, *line, range, set_block, token_modifiers);
            }
            Initialize {
                assignments,
                deferred: _,
                line,
                range,
            } => {
                handle_keyword(document, *line, range, token_modifiers);
                for assignment in assignments {
                    handle_assignment(
                        document,
                        *line,
                        assignment,
                        TokenTypes::Variable,
                        token_modifiers,
                    );
                }
            }
            _ => {}
        }
    }
}

fn handle_keyword(document: &mut Document, line: u32, range: &TokenRange, token_modifiers: u32) {
    document.semantic_tokens.insert(
        line,
        range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Keyword as u32,
            token_modifiers,
        },
    );
}

fn handle_assignment(
    document: &mut Document,
    line: u32,
    assignment: &Assignment,
    string_type: TokenTypes,
    token_modifiers: u32,
) {
    handle_variable_strings(
        document,
        line,
        &assignment.name,
        string_type,
        token_modifiers,
    );

    handle_values(document, line, &assignment.value, token_modifiers);

    if let Some(comment) = &assignment.comment {
        handle_comment(document, line, &comment, token_modifiers);
    }

    if assignment.value.to_string().trim().is_empty() {
        // TODO Add Warning
        let diag_start = Location {
            index: assignment.name.get_start_index(),
            line,
        };
        let end_index = if let Some(comment) = &assignment.comment {
            comment.get_end_index()
        } else {
            assignment.get_end_index()
        };
        let diag_end = Location {
            index: end_index,
            line,
        };
        let warning = new_warning_diagnostic(
            &diag_start,
            &diag_end,
            "Assignment with empty string.".to_string(),
        );
        document.diagnostics.push(warning);
    }
}

fn handle_quote(document: &mut Document, line: u32, quote: &Quote, token_modifiers: u32) {
    // Insert all of the variables first so they take priority
    quote.content.iter().for_each(|v| match v {
        behavior::tree::Value::Variable(variable) => {
            handle_variable(document, line, variable, token_modifiers)
        }
        _ => {}
    });

    // Then insert one large string for the span of the quote
    document.semantic_tokens.insert(
        line,
        quote.range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::String as u32,
            token_modifiers,
        },
    );
}

fn handle_variable_strings(
    document: &mut Document,
    line: u32,
    values: &VariableStrings,
    string_type: TokenTypes,
    token_modifiers: u32,
) {
    for v in values {
        match v {
            behavior::tree::VariableString::String(text, range) => {
                handle_string(document, line, text, range, string_type, token_modifiers);
            }
            behavior::tree::VariableString::Variable(variable) => {
                handle_variable(document, line, variable, token_modifiers);
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

fn handle_variable(document: &mut Document, line: u32, variable: &Variable, token_modifiers: u32) {
    match variable {
        behavior::tree::Variable::Regular { text: _, range }
        | behavior::tree::Variable::Partial { text: _, range } => {
            document.semantic_tokens.insert(
                line,
                range.clone(),
                SemanticTokenInfo {
                    token_type: TokenTypes::Variable as u32,
                    token_modifiers,
                },
            );
        }
    }
}

fn handle_comment(document: &mut Document, line: u32, comment: &Comment, token_modifiers: u32) {
    document.semantic_tokens.insert(
        line,
        comment.range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Comment as u32,
            token_modifiers,
        },
    );
}

fn handle_values(document: &mut Document, line: u32, values: &Values, token_modifiers: u32) {
    for v in values {
        match v {
            behavior::tree::Value::Boolean(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Type as u32,
                        token_modifiers,
                    },
                );
            }
            behavior::tree::Value::Integer(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers,
                    },
                );
            }
            behavior::tree::Value::Float(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers,
                    },
                );
            }
            behavior::tree::Value::String(_text, _range) => {
                // TODO: Should we color these differently than a quoted string?
                // For now, we'll leave these unformatted
            }
            behavior::tree::Value::Quote(quote) => {
                handle_quote(document, line, quote, token_modifiers)
            }
            behavior::tree::Value::Variable(variable) => {
                handle_variable(document, line, variable, token_modifiers)
            }
        }
    }
}

fn handle_behavior_block(
    document: &mut Document,
    line: u32,
    range: &TokenRange,
    behavior_block: &BehaviorBlock,
    token_modifiers: u32,
) {
    handle_keyword(document, line, range, token_modifiers);
    // TODO: Should check if the behavior name is in the path and display a
    // warning if it is not.
    handle_variable_strings(
        document,
        line,
        &behavior_block.behavior_name,
        TokenTypes::Struct,
        token_modifiers,
    );

    if let Some(comment) = &behavior_block.behavior_block_comment {
        handle_comment(document, line, &comment, token_modifiers);
    }

    // Prelude Comments
    if !behavior_block.prelude_comments.is_empty() {
        // NOTE: Invalid lines are handled by the parser. This should just
        // add comments.
        handle_lines(document, &behavior_block.prelude_comments, token_modifiers);
    }

    // Open Curly Comment
    if let Some(comment) = &behavior_block.open_curly_comment {
        handle_comment(
            document,
            behavior_block.open_curly_line,
            &comment,
            token_modifiers,
        );
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

    handle_lines(document, &behavior_block.body, token_modifiers);

    // TODO: This needs to be updated to copy case

    let inlay_text = format!("Behavior = {}", behavior_block.behavior_name.to_string());

    add_inlay_text(
        document,
        behavior_block.close_curly_line,
        behavior_block.close_curly_index + 1,
        inlay_text.as_str(),
    );

    if let Some(comment) = &behavior_block.close_curly_comment {
        handle_comment(
            document,
            behavior_block.close_curly_line,
            &comment,
            token_modifiers,
        );
    }
}

fn handle_unknown_block(
    document: &mut Document,
    line: u32,
    unknown_block: &UnknownBlock,
    token_modifiers: u32,
) {
    // Open Curly Comment
    if let Some(comment) = &unknown_block.open_curly_comment {
        handle_comment(
            document,
            unknown_block.open_curly_line,
            &comment,
            token_modifiers,
        );
    }

    // Add folding range for BehaviorBlock block
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = line;
    folding_range.end_line = unknown_block.close_curly_line - 1;
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    handle_lines(document, &unknown_block.body, token_modifiers);

    if let Some(comment) = &unknown_block.close_curly_comment {
        handle_comment(
            document,
            unknown_block.close_curly_line,
            &comment,
            token_modifiers,
        );
    }

    let diag_start = Location {
        index: unknown_block.open_curly_index,
        line: unknown_block.open_curly_line,
    };
    let diag_end = Location {
        index: unknown_block.close_curly_index + 1,
        line: unknown_block.close_curly_line,
    };
    let mut diagnostic_warning = new_warning_diagnostic(
        &diag_start,
        &diag_end,
        "Inactive block without a behavior name.".to_string(),
    );
    diagnostic_warning.tags = Some(vec![DiagnosticTag::UNNECESSARY]);
    document.diagnostics.push(diagnostic_warning);
}

fn handle_set_block(
    document: &mut Document,
    line: u32,
    range: &TokenRange,
    set_block: &SetBlock,
    token_modifiers: u32,
) {
    handle_keyword(document, line, range, token_modifiers);
    // TODO: Should check if the behavior name is in the path and display a
    // warning if it is not.
    handle_variable_strings(
        document,
        line,
        &set_block.mode_variable_name,
        TokenTypes::Struct,
        token_modifiers,
    );

    if let Some(comment) = &set_block.set_block_comment {
        handle_comment(document, line, &comment, token_modifiers);
    }

    // Prelude Comments
    if !set_block.prelude_comments.is_empty() {
        // NOTE: Invalid lines are handled by the parser. This should just
        // add comments.
        handle_lines(document, &set_block.prelude_comments, token_modifiers);
    }

    // Open Curly Comment
    if let Some(comment) = &set_block.open_curly_comment {
        handle_comment(
            document,
            set_block.open_curly_line,
            &comment,
            token_modifiers,
        );
    }

    // TODO: Add warning if SetBlock contains a initialize

    // Add folding range for SetBlock block
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = line;
    folding_range.end_line = set_block.close_curly_line;
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    // Handle Body.
    // TODO: Add lookup to see if assignments contain keywords

    handle_lines(document, &set_block.body, token_modifiers);

    // NOTE: I had the inlay hints enabled, but it gets confusing when there
    // is an else_value. I'm leaving them commented out for now since these
    // blocks typically are that long.

    // TODO: This needs to be updated to copy case
    // let inlay_text = format!(
    //     "Set {} = {}",
    //     set_block.mode_variable_name.to_string(),
    //     set_block.mode_value.to_string()
    // );

    // add_inlay_text(
    //     document,
    //     set_block.close_curly_line,
    //     set_block.close_curly_index + 1,
    //     inlay_text.as_str(),
    // );

    if let Some(comment) = &set_block.close_curly_comment {
        handle_comment(
            document,
            set_block.close_curly_line,
            &comment,
            token_modifiers,
        );
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
