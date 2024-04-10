use crate::cache::{Document, SemanticTokenInfo, TokenTypes};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentLink, FoldingRange, InlayHint, InlayHintKind,
    InlayHintLabel, Position,
};
use moos_parser::{
    lexers::{self, Location, TokenMap, TokenRange},
    moos::{
        self,
        error::{MoosParseError, MoosParseErrorKind},
        lexer::{State, Token},
        tree::{
            Assignment, Comment, Line, Lines, ProcessConfig, Quote, Values, Variable,
            VariableStrings,
        },
    },
    MoosParser, ParseError,
};
use tracing::{debug, error, info, trace, warn};

pub fn new_diagnostic(
    severity: DiagnosticSeverity,
    start: &Location,
    end: &Location,
    message: String,
) -> Diagnostic {
    Diagnostic::new(
        lsp_types::Range {
            start: (*start).into(),
            end: (*end).into(),
        },
        Some(severity),
        None,
        None,
        message,
        None,
        None,
    )
}

/// Helper method to create an error Diagnostic
pub fn new_error_diagnostic(start: &Location, end: &Location, message: String) -> Diagnostic {
    new_diagnostic(DiagnosticSeverity::ERROR, start, end, message)
}

pub fn new_warning_diagnostic(start: &Location, end: &Location, message: String) -> Diagnostic {
    new_diagnostic(DiagnosticSeverity::WARNING, start, end, message)
}

pub fn parse(document: &mut Document) {
    // NOTE: This clone is of a Rc<String>. It should not perform a deep copy
    // of the underlying String. This is needed to be able to pass a mutable
    // reference to the Document and the Result from the parser to other
    // functions.
    let text = document.text.clone();
    let lexer = moos_parser::MoosLexer::new(text.as_str());
    let mut state = moos_parser::moos::lexer::State::default();
    let result = MoosParser::new().parse(&mut state, text.as_str(), lexer);

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
                MoosParseErrorKind::MissingNewLine => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        String::from("Missing new line after application name."),
                    );
                    document.diagnostics.push(d);
                }
                MoosParseErrorKind::MissingTrailing(c) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Missing trailing character {c:?}"),
                    );
                    document.diagnostics.push(d);
                }
                MoosParseErrorKind::UnexpectedSymbol(c) => {}
                MoosParseErrorKind::UnexpectedComment(comment) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unexpected comment: {comment}"),
                    );
                    document.diagnostics.push(d);
                }
                MoosParseErrorKind::UnknownMacro(text) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unknown macro: {text}"),
                    );
                    document.diagnostics.push(d);
                }
                MoosParseErrorKind::MissingEndIf => {
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
    use moos_parser::moos::tree::Line::*;
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
            ProcessConfig {
                process_config,
                line,
                range,
            } => {
                // NOTE: A ProcessConfig inside another ProcessConfig is handled
                // by the parser.
                handle_process_config(document, *line, range, process_config);
            }
            Define {
                assignment,
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
        moos::tree::Value::Variable(variable) => handle_variable(document, line, variable),
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
            moos::tree::VariableString::String(text, range) => {
                handle_string(document, line, text, range, string_type, string_modifiers);
            }
            moos::tree::VariableString::Variable(variable) => {
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
        moos::tree::Variable::Regular { text: _, range }
        | moos::tree::Variable::Partial { text: _, range } => {
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
            moos::tree::Value::Boolean(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Type as u32,
                        token_modifiers: 0,
                    },
                );
            }
            moos::tree::Value::Integer(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            moos::tree::Value::Float(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            moos::tree::Value::String(_text, _range) => {
                // TODO: Should we color these differently than a quoted string?
                // For now, we'll leave these unformatted
            }
            moos::tree::Value::Quote(quote) => handle_quote(document, line, quote),
            moos::tree::Value::Variable(variable) => handle_variable(document, line, variable),
        }
    }
}

fn handle_process_config(
    document: &mut Document,
    line: u32,
    range: &TokenRange,
    process_config: &ProcessConfig,
) {
    handle_keyword(document, line, range);
    // TODO: Should check if the process name is in the path and display a
    // warning if it is not.
    handle_variable_strings(
        document,
        line,
        &process_config.process_name,
        TokenTypes::Struct,
        0,
    );

    if let Some(comment) = &process_config.process_config_comment {
        handle_comment(document, line, &comment);
    }

    // Prelude Comments
    if !process_config.prelude_comments.is_empty() {
        // NOTE: Invalid lines are handled by the parser. This should just
        // add comments.
        handle_lines(document, &process_config.prelude_comments);
    }

    // Open Curly Comment
    if let Some(comment) = &process_config.open_curly_comment {
        handle_comment(document, process_config.open_curly_line, &comment);
    }

    // TODO: Add warning if ProcessConfig contains a define

    // Add folding range for ProcessConfig block
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = line;
    folding_range.end_line = process_config.close_curly_line;
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    // Handle Body.
    // TODO: Add lookup to see if assignments contain keywords

    handle_lines(document, &process_config.body);

    let inlay_text = format!(
        "ProcessConfig = {}",
        process_config.process_name.to_string()
    );

    add_inlay_text(
        document,
        process_config.close_curly_line,
        process_config.close_curly_index + 1,
        inlay_text.as_str(),
    );

    if let Some(comment) = &process_config.close_curly_comment {
        handle_comment(document, process_config.close_curly_line, &comment);
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
