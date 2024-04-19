use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{
    notification::{DidChangeConfiguration, DidChangeTextDocument, DidOpenTextDocument},
    request::{
        Completion, DocumentLinkRequest, FoldingRangeRequest, Formatting, GotoDefinition,
        InlayHintRequest, InlineValueRequest, SemanticTokensFullRequest,
    },
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, DocumentLink, DocumentLinkParams, FoldingRange, FoldingRangeParams,
    GotoDefinitionResponse, InitializeParams, InlayHint, InlayHintParams, PublishDiagnosticsParams,
    SemanticTokens, SemanticTokensParams, SemanticTokensResult, TextEdit,
};

use crate::{cache::Project, workspace};
use tracing::debug as mlog;
use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

use lsp_server_derive_macro::{notification_handler, request_handler};

// Declare the Requests that we are going to handle.
#[request_handler]
enum MyRequests {
    Completion,
    DocumentLinkRequest,
    FoldingRangeRequest,
    Formatting,
    GotoDefinition,
    InlayHintRequest,
    InlineValueRequest,
    SemanticTokensFullRequest,
}

// Declare the Notifications we are going to handle.
#[notification_handler]
enum MyNotifications {
    DidChangeTextDocument,
    DidOpenTextDocument,
    DidChangeConfiguration,
}

pub struct Handler {
    cache: Arc<Mutex<Project>>,
    connection: Connection,
    params: InitializeParams,
}

impl Handler {
    pub fn new(connection: Connection, params: InitializeParams) -> Self {
        let root = params.root_path.clone().unwrap_or_default();

        let cache = Arc::new(Mutex::new(Project::new(root)));

        // TODO: Need a better way to scan the entire workspace.
        if let Some(root_path) = params.root_path.clone() {
            if let Some(root_uri) = params.root_uri.clone() {
                workspace::scan_workspace(&connection, cache.clone(), root_path, root_uri);
            }
        }

        Self {
            cache,
            connection,
            params,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        let receiver = self.connection.receiver.clone();
        for msg in receiver {
            trace!("got msg: {msg:?}");
            match msg {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    if let Some(response) = self.handle_request(request) {
                        trace!("Sending Response: {response:?}");
                        self.connection.sender.send(Message::Response(response))?;
                    }
                }
                Message::Response(resp) => {
                    mlog!("got response: {resp:?}");
                }
                Message::Notification(notification) => {
                    mlog!("got notification: {notification:?}");
                    if let Err(e) = self.handle_notification(notification) {
                        error!("Failed to handle notification: {e:?}");
                    }
                }
            }
        }
        Ok(())
    }

    //-----------------------------------------------------------------------------
    // Requests
    //-----------------------------------------------------------------------------
    pub fn handle_request(&mut self, request: lsp_server::Request) -> Option<Response> {
        use MyRequests::*;
        info!("Got request: {:?}", request.method);
        match MyRequests::from(request) {
            Completion(id, params) => {
                mlog!("Got completion request #{id}: {params:?}");
                let result = self.handle_completion(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            DocumentLinkRequest(id, params) => {
                //
                let result = self.handle_document_link_request(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            GotoDefinition(id, params) => {
                mlog!("got gotoDefinition request #{id}: {params:?}");
                let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            FoldingRangeRequest(id, params) => {
                let result = self.handle_folding_range_request(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            InlineValueRequest(id, params) => {
                // TODO:
                warn!("Received Unhandled InlineValueRequest: {params:?}");
            }
            InlayHintRequest(id, params) => {
                mlog!("Got InlayHintRequest: {id} {params:?}");
                let result = self.handle_inlay_hint_request(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            SemanticTokensFullRequest(id, params) => {
                mlog!("Got SemanticTokensFullRequest: {id} {params:?}");
                let result = self.handle_semantic_tokens_full(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            Formatting(id, params) => {
                mlog!("Got Formatting Request: {id} {params:?}");
                let result = self.handle_document_formatting(&id, params);
                let result = serde_json::to_value(&result).unwrap();
                let response = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Some(response);
            }
            Unhandled(req) => info!("Unhandled Request {:?}", req.method),
            Error { method, error } => {
                error!("Failed to handle Request {method}: {error:?}")
            }
        }
        None
    }

    fn handle_semantic_tokens_full(
        &mut self,
        id: &RequestId,
        params: SemanticTokensParams,
    ) -> Option<SemanticTokensResult> {
        let uri = params.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                // Loop through the tokens and convert them
                let tokens = SemanticTokens {
                    result_id: None,
                    data: doc.get_semantic_tokens().data,
                };
                trace!("Semantic Tokens for {uri} {tokens:?}");
                return Some(SemanticTokensResult::Tokens(tokens));
            }
        }

        return None;
    }

    fn handle_folding_range_request(
        &mut self,
        id: &RequestId,
        params: FoldingRangeParams,
    ) -> Option<Vec<FoldingRange>> {
        let uri = params.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                if !doc.folding_ranges.is_empty() {
                    return Some(doc.folding_ranges.clone());
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        return None;
    }

    fn handle_document_link_request(
        &mut self,
        id: &RequestId,
        params: DocumentLinkParams,
    ) -> Option<Vec<DocumentLink>> {
        let uri = params.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                if !doc.document_links.is_empty() {
                    return Some(doc.document_links.clone());
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        return None;
    }

    fn handle_completion(
        &mut self,
        id: &RequestId,
        params: CompletionParams,
    ) -> Option<CompletionResponse> {
        let uri = params.text_document_position.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                //
                let position = params.text_document_position.position;
                return doc.get_completion(position, params.context);
            }
        }

        None
    }

    fn handle_inlay_hint_request(
        &mut self,
        id: &RequestId,
        params: InlayHintParams,
    ) -> Option<Vec<InlayHint>> {
        let uri = params.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                if !doc.inlay_hints.is_empty() {
                    let is_inside =
                        |range: &lsp_types::Range, position: &lsp_types::Position| -> bool {
                            position.line >= range.start.line && position.line <= range.end.line
                        };

                    return Some(
                        doc.inlay_hints
                            .iter()
                            .filter(|&hint| is_inside(&params.range, &hint.position))
                            .cloned()
                            .collect(),
                    );
                }
            }
        }
        return None;
    }

    fn handle_document_formatting(
        &mut self,
        id: &RequestId,
        params: DocumentFormattingParams,
    ) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;

        if let Ok(cache) = self.cache.lock() {
            if let Some(doc) = cache.documents.get(&uri) {
                return doc.get_formats(&params.options);
            }
        }

        return None;
    }

    //-----------------------------------------------------------------------------
    // Notifications
    //-----------------------------------------------------------------------------
    pub fn handle_notification(
        &mut self,
        notification: lsp_server::Notification,
    ) -> anyhow::Result<()> {
        use MyNotifications::*;
        match MyNotifications::from(notification) {
            DidChangeConfiguration(params) => {
                info!("Configuration Changed: {params:?}")
            }
            DidChangeTextDocument(params) => return self.handle_did_change_text_document(params),
            DidOpenTextDocument(params) => return self.handle_did_open_text_document(params),
            Unhandled(n) => info!("Unhandled Notification: {:?}", n.method),
            Error { method, error } => error!("Failed to handle Notification {method}: {error:?}"),
        }

        Ok(())
    }

    pub fn handle_did_open_text_document(
        &mut self,
        params: DidOpenTextDocumentParams,
    ) -> anyhow::Result<()> {
        let uri = &params.text_document.uri;
        let language_id = &params.text_document.language_id;
        info!("Document Opened: {uri} {language_id}");

        if let Ok(cache) = &mut self.cache.lock() {
            let document = cache.insert(&uri, params.text_document.text);

            let diagnostics =
                PublishDiagnosticsParams::new(uri.clone(), document.diagnostics.clone(), None);
            let params = serde_json::to_value(&diagnostics).unwrap();
            use lsp_types::notification::Notification;
            let notification = lsp_server::Notification {
                method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                params,
            };
            self.connection
                .sender
                .send(Message::Notification(notification))?;

            return Ok(());
        } else {
            return Err(anyhow::Error::msg("Failed to acquire project lock."));
        }
    }

    pub fn handle_did_change_text_document(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> anyhow::Result<()> {
        let uri = &params.text_document.uri;
        // We are only handling full changes so don't do anything with version
        //let version = params.text_document.version;
        info!("Document Changed: {uri:?}");
        if params.content_changes.len() != 1 {
            return Err(anyhow::Error::msg(format!(
                "Text document changes expected to be full for document: {}",
                uri
            )));
        }

        for change in params.content_changes {
            if let Some(range_length) = change.range_length {
                return Err(anyhow::Error::msg(format!(
                    "Received deprecated range_length for document: {} of range: {}",
                    uri, range_length
                )));
            }

            if let Some(range) = change.range {
                return Err(anyhow::Error::msg(format!(
                    "Received unexpected range for document: {} of range: {:?}",
                    uri, range
                )));
            }

            if let Ok(cache) = &mut self.cache.lock() {
                let document = cache.insert(&uri, change.text);

                let diagnostics =
                    PublishDiagnosticsParams::new(uri.clone(), document.diagnostics.clone(), None);
                let params = serde_json::to_value(&diagnostics).unwrap();
                use lsp_types::notification::Notification;
                let notification = lsp_server::Notification {
                    method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                    params,
                };

                let sender = self.connection.sender.clone();
                self.connection
                    .sender
                    .send(Message::Notification(notification))?;
            } else {
                return Err(anyhow::Error::msg("Failed to acquire cache lock"));
            }
        }

        return Ok(());
    }
}
