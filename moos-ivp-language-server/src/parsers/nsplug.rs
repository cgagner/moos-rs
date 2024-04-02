use crate::cache::{Document, SemanticTokenInfo, TokenTypes};
use lsp_types::{Diagnostic, DiagnosticSeverity, FoldingRange};
use moos_parser::{
    lexers::{self, Location, TokenMap, TokenRange},
    nsplug::{
        self,
        error::{PlugParseError, PlugParseErrorKind},
        lexer::{State, Token},
        tree::{
            IfDefBranch, IfNotDefBranch, IncludePath, IncludeTag, Line, Lines, MacroCondition,
            MacroDefinition, MacroType, Quote, Values, Variable, VariableStrings,
        },
    },
    ParseError, PlugParser,
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
    new_diagnostic(DiagnosticSeverity::ERROR, start, end, message)
}

pub fn parse(document: &mut Document) {
    // TODO: We should be able to parse the document without cloning the text,
    // but this breaks the borrow checker
    let text = document.text.clone();
    let mut lexer = moos_parser::nsplug::lexer::Lexer::new(text.as_str());
    let mut state = moos_parser::nsplug::lexer::State::default();
    let result = PlugParser::new().parse(&mut state, text.as_str(), lexer);

    info!("Parse Results: {result:?}");

    if let Ok(lines) = result {
        handle_lines(document, &lines);
    }

    let iter = document.diagnostics.iter();
    state.errors.iter().for_each(|e| {
        error!("Parse Error: {e:?}");
    });
    // TODO: Add new method to handle converting errors into diagnostics
    // TODO: Only create diagnostics if the client supports diagnostics
    // TODO: Need to handle dropped tokens
    for e in state.errors {
        match e.error {
            ParseError::User { error } => match error.kind {
                PlugParseErrorKind::MissingNewLine => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        String::from("Missing new line after application name."),
                    );
                    document.diagnostics.push(d);
                }
                PlugParseErrorKind::MissingTrailing(c) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Missing trailing character {c:?}"),
                    );
                    document.diagnostics.push(d);
                }
                PlugParseErrorKind::UnexpectedSymbol(c) => {}
                PlugParseErrorKind::UnexpectedComment(comment) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unexpected comment: {comment}"),
                    );
                    document.diagnostics.push(d);
                }
                PlugParseErrorKind::UnknownMacro(text) => {
                    let d = new_error_diagnostic(
                        &error.loc_start,
                        &error.loc_end,
                        format!("Unknown macro: {text}"),
                    );
                    document.diagnostics.push(d);
                }
                PlugParseErrorKind::MissingEndIf => {
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
    use moos_parser::nsplug::tree::Line::*;
    use moos_parser::nsplug::tree::MacroType;
    for l in lines {
        match l {
            Macro {
                macro_type,
                comment,
                line,
            } => {
                let line = *line;
                match macro_type {
                    MacroType::Define { definition, range } => {
                        // TODO: Add declaration, definition, and references
                        handle_macro_token(document, line, &range);
                        handle_variable_strings(
                            document,
                            line,
                            &definition.name,
                            TokenTypes::Variable,
                            0,
                        );
                        handle_values(document, line, &definition.value);
                    }
                    MacroType::Include { path, tag, range } => {
                        handle_include(document, line, &path, &tag, &range);
                    }

                    MacroType::IfDef {
                        condition,
                        branch,
                        body,
                        range,
                    } => {
                        let mut folding_range = FoldingRange::default();
                        folding_range.start_line = line;
                        folding_range.end_line = branch.get_start_line() - 1;
                        folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
                        document.add_folding_range(folding_range);

                        handle_macro_token(document, line, &range);
                        handle_macro_condition(document, line, &condition);
                        handle_lines(document, body);
                        handle_ifdef_branch(document, line, branch);
                    }
                    MacroType::IfNotDef {
                        clauses,
                        branch,
                        body,
                        range,
                    } => {
                        let mut folding_range = FoldingRange::default();
                        folding_range.start_line = line;
                        folding_range.end_line = branch.get_start_line() - 1;
                        folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
                        document.add_folding_range(folding_range);

                        handle_macro_token(document, line, &range);
                        for clause in clauses {
                            handle_variable_strings(
                                document,
                                line,
                                clause,
                                TokenTypes::Variable,
                                0,
                            );
                        }
                        handle_lines(document, body);
                        handle_ifndef_branch(document, line, branch);
                    }
                }
            }
            Variable { variable, line } => {
                // TODO: Add a reference to the variable

                handle_variable(document, *line, &variable);
            }
            _ => {}
        }
    }
}

fn handle_macro_token(document: &mut Document, line: u32, range: &TokenRange) {
    document.semantic_tokens.insert(
        line,
        range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Keyword as u32,
            token_modifiers: 0,
        },
    );
}

fn handle_include(
    document: &mut Document,
    line: u32,
    path: &IncludePath,
    tag: &Option<IncludeTag>,
    range: &TokenRange,
) {
    document.semantic_tokens.insert(
        line,
        range.clone(),
        SemanticTokenInfo {
            token_type: TokenTypes::Macro as u32,
            token_modifiers: 0,
        },
    );
    match path {
        IncludePath::VariableStrings(values, _range) => {
            handle_variable_strings(document, line, &values, TokenTypes::String, 0);
        }
        IncludePath::Quote(quote) => handle_quote(document, line, &quote),
    }

    if let Some(tag) = tag {
        document.semantic_tokens.insert(
            line,
            tag.range.clone(),
            SemanticTokenInfo {
                token_type: TokenTypes::Namespace as u32,
                token_modifiers: 0,
            },
        );
    }
}

fn handle_quote(document: &mut Document, line: u32, quote: &Quote) {
    // Insert all of the variables first so they take priority
    quote.content.iter().for_each(|v| match v {
        nsplug::tree::VariableString::Variable(variable) => {
            handle_variable(document, line, variable)
        }
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
            nsplug::tree::VariableString::String(text, range) => {
                handle_string(document, line, text, range, string_type, string_modifiers);
            }
            nsplug::tree::VariableString::Variable(variable) => {
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
        nsplug::tree::Variable::Regular { text: _, range }
        | nsplug::tree::Variable::Upper { text: _, range }
        | nsplug::tree::Variable::Partial { text: _, range }
        | nsplug::tree::Variable::PartialUpper { text: _, range } => {
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

fn handle_values(document: &mut Document, line: u32, values: &Values) {
    for v in values {
        match v {
            nsplug::tree::Value::Boolean(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Type as u32,
                        token_modifiers: 0,
                    },
                );
            }
            nsplug::tree::Value::Integer(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            nsplug::tree::Value::Float(_value, _value_str, range) => {
                document.semantic_tokens.insert(
                    line,
                    range.clone(),
                    SemanticTokenInfo {
                        token_type: TokenTypes::Number as u32,
                        token_modifiers: 0,
                    },
                );
            }
            nsplug::tree::Value::String(_text, _range) => {
                // TODO: Should we color these differently than a quoted string?
                // For now, we'll leave these unformatted
            }
            nsplug::tree::Value::Quote(quote) => handle_quote(document, line, quote),
            nsplug::tree::Value::Variable(variable) => handle_variable(document, line, variable),
        }
    }
}

fn handle_macro_definition(document: &mut Document, line: u32, definition: &MacroDefinition) {
    handle_variable_strings(document, line, &definition.name, TokenTypes::Variable, 0);
    handle_values(document, line, &definition.value);
}

fn handle_macro_condition(document: &mut Document, line: u32, condition: &MacroCondition) {
    match condition {
        MacroCondition::Simple(definition) => handle_macro_definition(document, line, definition),
        MacroCondition::Disjunction {
            operator_range,
            lhs,
            rhs,
        }
        | MacroCondition::Conjunction {
            operator_range,
            lhs,
            rhs,
        } => {
            handle_macro_definition(document, line, lhs);
            document.semantic_tokens.insert(
                line,
                operator_range.clone(),
                SemanticTokenInfo {
                    token_type: TokenTypes::Operator as u32,
                    token_modifiers: 0,
                },
            );
            handle_macro_condition(document, line, rhs);
        }
    }
}

fn handle_ifdef_branch(document: &mut Document, _parent_line: u32, input_branch: &IfDefBranch) {
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = input_branch.get_start_line();
    folding_range.end_line = input_branch.get_end_line();
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    match input_branch {
        IfDefBranch::ElseIfDef {
            line,
            macro_range,
            condition,
            body,
            branch,
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_macro_condition(document, *line, &condition);
            handle_lines(document, body);
            handle_ifdef_branch(document, *line, branch);
        }
        IfDefBranch::Else {
            line,
            macro_range,
            body,
            endif_line,
            endif_macro_range,
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_lines(document, body);
            handle_macro_token(document, *endif_line, &endif_macro_range);
        }
        IfDefBranch::EndIf { line, macro_range } => {
            handle_macro_token(document, *line, &macro_range);
        }
    }
}

fn handle_ifndef_branch(document: &mut Document, _parent_line: u32, input_branch: &IfNotDefBranch) {
    let mut folding_range = FoldingRange::default();
    folding_range.start_line = input_branch.get_start_line();
    folding_range.end_line = input_branch.get_end_line();
    folding_range.kind = Some(lsp_types::FoldingRangeKind::Region);
    // Adding to the document will check if the folding range is valid.
    document.add_folding_range(folding_range);

    match input_branch {
        IfNotDefBranch::Else {
            line,
            macro_range,
            body,
            endif_line,
            endif_macro_range,
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_lines(document, body);
            handle_macro_token(document, *endif_line, &endif_macro_range);
        }
        IfNotDefBranch::EndIf { line, macro_range } => {
            handle_macro_token(document, *line, &macro_range);
        }
    }
}

/*

TODO: These are reminders of tokens that we should be handling.

Token::Comment(_comment) => Some(SemanticTokenInfo {
    token_type: TokenTypes::Comment as u32,
    token_modifiers: 0,
}),

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
*/
