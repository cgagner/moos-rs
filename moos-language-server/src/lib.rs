use lsp_types::SemanticToken;
use moos_parser::lexer::State;
use wasm_bindgen::convert::ReturnWasmAbi;

use core::result::Result;
use moos_parser;
use moos_parser::error::{MoosParseError, MoosParseErrorKind};
use std::hash::Hash;
use std::str::FromStr;
use std::vec;

use serde_wasm_bindgen::{from_value, to_value, Error};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, str::Split};
use wasm_bindgen::prelude::*;

pub mod helpers;
use helpers::LspLogger;

pub mod cache;
use cache::Cache;

use lalrpop_util::ParseError;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidDeleteFiles,
        DidOpenTextDocument, DidSaveTextDocument, Initialized, Notification,
    },
    request::{Request, SemanticTokensFullRequest},
    DeleteFilesParams, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidOpenTextDocumentParams, FileChangeType, FileDelete, FileEvent,
    NumberOrString, PublishDiagnosticsParams, SemanticTokens, SemanticTokensParams,
    SemanticTokensRangeResult, SemanticTokensResult, TextDocumentContentChangeEvent,
    TextDocumentItem, Url, VersionedTextDocumentIdentifier,
};

#[wasm_bindgen]
pub struct MoosLanguageServer {
    send_diagnostics_callback: js_sys::Function,
    cache: HashMap<String, Cache>,
    // TODO: Create a HashSet of keywords for MOOS Apps and Behaviors
}

#[wasm_bindgen]
impl MoosLanguageServer {
    #[wasm_bindgen(js_class = MoosLanguageServer, js_name = getTokenTypes)]
    pub fn get_token_types() -> js_sys::Array {
        cache::TOKEN_TYPES
            .into_iter()
            .map(|x| JsValue::from_str(x))
            .collect::<js_sys::Array>()
    }

    #[wasm_bindgen(js_class = MoosLanguageServer, js_name = getTokenModifiers)]
    pub fn get_token_modifiers() -> js_sys::Array {
        cache::TOKEN_MODIFIERS
            .into_iter()
            .map(|x| JsValue::from_str(x))
            .collect::<js_sys::Array>()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(send_diagnostics_callback: &js_sys::Function) -> Self {
        LspLogger::init(log::LevelFilter::Debug);
        log::info!("MoosLanguageServer Created");

        Self {
            send_diagnostics_callback: send_diagnostics_callback.clone(),
            cache: HashMap::new(),
        }
    }

    #[wasm_bindgen(js_class = MoosLanguageServer, js_name = onNotification)]
    pub fn on_notification(&mut self, method: &str, params: JsValue) {
        log::debug!(
            "on_notification: {}({})",
            method,
            params.as_string().unwrap_or(String::new())
        );

        match method {
            DidOpenTextDocument::METHOD => {
                // TODO: Don't unwrap without catching

                // let document: DidOpenTextDocumentParams = from_value(params).unwrap();
                // log::info!("{:?}", document);
                let DidOpenTextDocumentParams { text_document } = from_value(params).unwrap();
                self.on_did_open(text_document);
            }
            DidChangeTextDocument::METHOD => {
                let DidChangeTextDocumentParams {
                    text_document,
                    content_changes,
                } = from_value(params).unwrap();

                self.on_did_change(text_document, content_changes);
            }
            DidChangeWatchedFiles::METHOD => {
                let DidChangeWatchedFilesParams { changes } = from_value(params).unwrap();
            }
            DidDeleteFiles::METHOD => {
                let DeleteFilesParams { files } = from_value(params).unwrap();
            }
            DidSaveTextDocument::METHOD => (),
            DidCloseTextDocument::METHOD => (),
            Initialized::METHOD => (),
            _ => log::info!("Unexpected notification"),
        }
    }

    #[wasm_bindgen(js_class = MoosLanguageServer, js_name = onSemanticTokensFull)]
    pub fn on_semantic_tokens_full(&mut self, params: JsValue) -> Result<JsValue, Error> {
        log::info!("on_semantic_tokens_full: {:?}", params);
        let SemanticTokensParams {
            text_document,
            partial_result_params,
            work_done_progress_params,
        } = from_value(params).unwrap();
        log::info!(
            "SemanticTokensFullRequest {} {:?} {:?}",
            text_document.uri,
            partial_result_params.partial_result_token,
            work_done_progress_params.work_done_token
        );

        let mut tokens = SemanticTokens {
            result_id: None,
            data: self.cache[&text_document.uri.to_string()]
                .semantic_tokens
                .clone(),
        };

        log::info!("Semantic Tokens: {:?}", tokens);
        to_value(&SemanticTokensResult::from(tokens))
    }

    #[wasm_bindgen(js_class = MoosLanguageServer, js_name = onRequest)]
    pub fn on_request(&mut self, method: &str, params: JsValue, token: JsValue) {
        log::debug!(
            "on_request: {}({}) - token: {}",
            method,
            params.as_string().unwrap_or(String::new()),
            token.as_string().unwrap_or(String::new())
        );

        match method {
            // SemanticTokensFullRequest::METHOD => {
            //     let SemanticTokensParams {
            //         text_document,
            //         partial_result_params,
            //         work_done_progress_params,
            //     } = from_value(params).unwrap();
            //     log::info!(
            //         "SemanticTokensFullRequest {} {:?} {:?}",
            //         text_document.uri,
            //         partial_result_params.partial_result_token,
            //         work_done_progress_params.work_done_token
            //     );
            // }
            _ => log::info!("Unexpected request: {}", method),
        }
    }

    fn on_did_open(&mut self, document: TextDocumentItem) {
        log::debug!(
            "on_did_open: {}\n\tLanguage: {}\n\tVersion: {}",
            document.uri,
            document.language_id,
            document.version
        );
        let input = document.text.as_str();
        let mut c = Cache::new();

        let mut state = State::default();
        let mut listener = cache::MoosTokenListener::new(&mut c);
        let mut lexer = moos_parser::Lexer::new(input);
        lexer.add_listener(&mut listener);

        let result = moos_parser::LinesParser::new().parse(&mut state, input, lexer);

        // TODO: Need to write a helper to convert between MoosParseErrors
        // and diagnostics
        for e in state.errors {
            log::error!("Found error when parsing: {:?}", e);

            match e.error {
                ParseError::User { error } => match error.kind {
                    MoosParseErrorKind::InvalidConfigBlock => {}
                    MoosParseErrorKind::MissingNewLine => {
                        let d = Diagnostic::new(
                            lsp_types::Range {
                                start: lsp_types::Position {
                                    line: error.loc_start.line as u32,
                                    character: error.loc_start.index as u32,
                                },
                                end: lsp_types::Position {
                                    line: error.loc_end.line as u32,
                                    character: error.loc_end.index as u32,
                                },
                            },
                            Some(DiagnosticSeverity::ERROR),
                            None,
                            None,
                            String::from("Missing new line after application name."),
                            None,
                            None,
                        );
                        c.diagnostics.push(d);
                    }
                    MoosParseErrorKind::MissingTrailing(c) => {}
                    MoosParseErrorKind::UnexpectedSymbol(c) => {}
                    _ => {}
                },
                ParseError::UnrecognizedToken { token, expected } => {
                    let (loc_start, token, loc_end) = token;
                    let d = Diagnostic::new(
                        lsp_types::Range {
                            start: lsp_types::Position {
                                line: loc_start.line as u32,
                                character: loc_start.index as u32,
                            },
                            end: lsp_types::Position {
                                line: loc_end.line as u32,
                                character: loc_end.index as u32,
                            },
                        },
                        Some(DiagnosticSeverity::ERROR),
                        None,
                        None,
                        format!("Unrecognized token: {:?}. Expected: {:?}", token, expected),
                        None,
                        None,
                    );
                    c.diagnostics.push(d);
                }
                _ => {}
            }
        }
        log::warn!("Parse Result: {:?}", result);
        if let Err(e) = result {
            log::error!("Found an error when parsing: {:?}", e);
        }
        // TODO: Need to cleanup the diagnostics stuff.. Seems to be working.

        let pd = PublishDiagnosticsParams::new(document.uri.clone(), c.diagnostics.clone(), None);

        self.cache.insert(document.uri.to_string(), c);
        let this = &JsValue::null();
        let r = self
            .send_diagnostics_callback
            .call1(this, &to_value(&pd).unwrap());
    }

    fn on_did_change(
        &mut self,
        document: VersionedTextDocumentIdentifier,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        log::debug!(
            "on_did_change: {}\n\tVersion: {}",
            document.uri,
            document.version
        );

        // TODO: This should be updated to handle incremental changes
        if changes.len() == 1 {
            let input = changes.first().unwrap().text.as_str();
            let mut c = Cache::new();

            let mut listener = cache::MoosTokenListener::new(&mut c);
            let mut lexer = moos_parser::Lexer::new(input);
            lexer.add_listener(&mut listener);
            let mut state = State::default();
            let result = moos_parser::LinesParser::new().parse(&mut state, input, lexer);

            // TODO: Need to write a helper to convert between MoosParseErrors
            // and diagnostics
            for e in state.errors {
                log::error!("Found error when parsing: {:?}", e);

                match e.error {
                    ParseError::User { error } => match error.kind {
                        MoosParseErrorKind::InvalidConfigBlock => {}
                        MoosParseErrorKind::MissingNewLine => {
                            let d = Diagnostic::new(
                                lsp_types::Range {
                                    start: lsp_types::Position {
                                        line: error.loc_start.line as u32,
                                        character: error.loc_start.index as u32,
                                    },
                                    end: lsp_types::Position {
                                        line: error.loc_end.line as u32,
                                        character: error.loc_end.index as u32,
                                    },
                                },
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                None,
                                String::from("Missing new line after application name."),
                                None,
                                None,
                            );
                            c.diagnostics.push(d);
                        }
                        MoosParseErrorKind::MissingTrailing(c) => {}
                        MoosParseErrorKind::UnexpectedSymbol(c) => {}
                        _ => {}
                    },
                    ParseError::UnrecognizedToken { token, expected } => {
                        let (loc_start, token, loc_end) = token;
                        let d = Diagnostic::new(
                            lsp_types::Range {
                                start: lsp_types::Position {
                                    line: loc_start.line as u32,
                                    character: loc_start.index as u32,
                                },
                                end: lsp_types::Position {
                                    line: loc_end.line as u32,
                                    character: loc_end.index as u32,
                                },
                            },
                            Some(DiagnosticSeverity::ERROR),
                            None,
                            None,
                            format!("Unrecognized token: {:?}. Expected: {:?}", token, expected),
                            None,
                            None,
                        );
                        c.diagnostics.push(d);
                    }
                    _ => {}
                }
            }
            log::warn!("Parse Result: {:?}", result);
            if let Err(e) = result {
                log::error!("Found an error when parsing: {:?}", e);
            }
            // TODO: Need to cleanup the diagnostics stuff.. Seems to be working.

            let pd =
                PublishDiagnosticsParams::new(document.uri.clone(), c.diagnostics.clone(), None);

            self.cache.insert(document.uri.to_string(), c);
            let this = &JsValue::null();
            let r = self
                .send_diagnostics_callback
                .call1(this, &to_value(&pd).unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
