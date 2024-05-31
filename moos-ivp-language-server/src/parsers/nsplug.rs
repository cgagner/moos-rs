use std::{ops::Deref, sync::Arc};

use crate::cache::{Document, SemanticTokenInfo, TokenTypes};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentLink, FoldingRange, InlayHint, InlayHintKind,
    InlayHintLabel, Position, TextEdit,
};
use moos_parser::{
    base::TreeNode,
    lexers::{self, Location, TokenMap, TokenRange},
    nsplug::{
        self,
        error::{PlugParseError, PlugParseErrorKind},
        lexer::{State, Token},
        tree::{
            IfDefBranch, IfNotDefBranch, IncludePath, IncludeTag, Line, Lines, MacroCondition,
            MacroDefinition, MacroType, Quote, Values, Variable, VariableStrings, ENDIF_STR,
        },
    },
    FormatOptions, ParseError, PlugParser, TextFormatter,
};
use tracing::{debug, error, info, trace, warn};

use super::{find_relative_file, new_error_diagnostic};

const INVALID_CONDITION_STR: &str = "Invalid conditions. #ifdef conditions cannot contain both disjunction (logical-or) and conjunction (logical-and) operators.";

pub fn format(document: &Document, format_options: &FormatOptions) -> Option<Vec<TextEdit>> {
    let lines = &document.plug_lines;

    if !lines.is_empty() {
        let edits: Vec<TextEdit> = lines
            .iter()
            .map(|line| line.format(&format_options, 0))
            .flatten()
            .collect();
        if !edits.is_empty() {
            return Some(edits);
        }
    }

    return None;
}

pub fn parse(document: &mut Document) {
    // NOTE: This clone is of a Arc<str>. It should not perform a deep copy
    // of the underlying str. This is needed to be able to pass a mutable
    // reference to the Document and the Result from the parser to other
    // functions.
    let text = Arc::clone(&document.text);
    let lexer = moos_parser::nsplug::lexer::Lexer::new(text.deref());
    let mut state = moos_parser::nsplug::lexer::State::default();
    let result = PlugParser::new().parse(&mut state, text.deref(), lexer);

    info!("Parse Results: {result:?}");

    if let Ok(lines) = result {
        handle_lines(document, &lines);
        document.plug_lines = lines;
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
                PlugParseErrorKind::UnexpectedSymbol(_) => {}
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
                        format!("Missing {ENDIF_STR}"),
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
    for l in lines {
        match l {
            Macro {
                macro_type,
                comment: _,
                line,
                line_end_index: _,
                indent: _,
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
                        if !condition.is_valid() {
                            let start = Location {
                                line,
                                index: condition.get_start_index(),
                            };
                            let end = Location {
                                line,
                                index: condition.get_end_index(),
                            };
                            let d = new_error_diagnostic(
                                &start,
                                &end,
                                INVALID_CONDITION_STR.to_string(),
                            );
                            document.diagnostics.push(d);
                        }
                        handle_lines(document, body);

                        let parent_text = format!(" {}", macro_type.to_string());
                        handle_ifdef_branch(document, line, branch, parent_text.as_str());
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

                        let parent_text = format!(" {}", macro_type.to_string());
                        handle_ifndef_branch(document, line, branch, parent_text.as_str());
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
            token_type: TokenTypes::Keyword as u32,
            token_modifiers: 0,
        },
    );
    match path {
        IncludePath::VariableStrings(values, _range) => {
            handle_variable_strings(document, line, &values, TokenTypes::String, 0);
        }
        IncludePath::Quote(quote) => handle_quote(document, line, &quote),
    }

    // TODO: This should really use the workspace, but for now we'll just
    // handle this using the local file system.
    if let Some(include_url) = find_relative_file(&document.uri, path.to_string().as_str()) {
        let include_range = path.get_token_range();
        document.document_links.push(DocumentLink {
            range: lsp_types::Range {
                start: Position {
                    line,
                    character: include_range.start,
                },
                end: Position {
                    line,
                    character: include_range.end,
                },
            },
            target: Some(include_url),
            tooltip: None,
            data: None,
        });
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
        nsplug::tree::Value::Variable(variable) => handle_variable(document, line, variable),
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

fn handle_ifdef_branch(
    document: &mut Document,
    _parent_line: u32,
    input_branch: &IfDefBranch,
    parent_text: &str,
) {
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
            ..
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_macro_condition(document, *line, &condition);
            if !condition.is_valid() {
                let start = Location {
                    line: *line,
                    index: condition.get_start_index(),
                };
                let end = Location {
                    line: *line,
                    index: condition.get_end_index(),
                };
                let d = new_error_diagnostic(&start, &end, INVALID_CONDITION_STR.to_string());
                document.diagnostics.push(d);
            }

            handle_lines(document, body);
            handle_ifdef_branch(document, *line, branch, parent_text);
        }
        IfDefBranch::Else {
            line,
            macro_range,
            body,
            endif_line,
            endif_macro_range,
            ..
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_lines(document, body);
            handle_macro_token(document, *endif_line, &endif_macro_range);
            add_inlay_text(document, *endif_line, endif_macro_range.end, parent_text);
        }
        IfDefBranch::EndIf {
            line, macro_range, ..
        } => {
            handle_macro_token(document, *line, &macro_range);
            add_inlay_text(document, *line, macro_range.end, parent_text);
        }
    }
}

fn handle_ifndef_branch(
    document: &mut Document,
    _parent_line: u32,
    input_branch: &IfNotDefBranch,
    parent_text: &str,
) {
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
            ..
        } => {
            handle_macro_token(document, *line, &macro_range);
            handle_lines(document, body);
            handle_macro_token(document, *endif_line, &endif_macro_range);
            add_inlay_text(document, *endif_line, endif_macro_range.end, parent_text);
        }
        IfNotDefBranch::EndIf {
            line, macro_range, ..
        } => {
            handle_macro_token(document, *line, &macro_range);
            add_inlay_text(document, *line, macro_range.end, parent_text);
        }
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
