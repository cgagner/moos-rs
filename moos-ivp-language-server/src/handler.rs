use std::error::Error;

use lsp_server::{Connection, Message, Notification, RequestId, Response};
use lsp_types::{
    notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidOpenTextDocument, PublishDiagnostics,
    },
    request::{Completion, FoldingRangeRequest, GotoDefinition, SemanticTokensFullRequest},
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, FoldingRange, FoldingRangeParams,
    GotoDefinitionResponse, InitializeParams, OneOf, PublishDiagnosticsParams, SemanticTokens,
    SemanticTokensParams, SemanticTokensResult, ServerCapabilities, TextDocumentContentChangeEvent,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};

use crate::cache::Project;
use tracing::debug as mlog;
use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

use lsp_server_derive_macro::{notification_handler, request_handler};

// Declare the Requests that we are going to handle.
#[request_handler]
enum MyRequests {
    Completion,
    FoldingRangeRequest,
    GotoDefinition,
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
    cache: Project,
    connection: Connection,
    params: InitializeParams,
}

impl Handler {
    pub fn new(connection: Connection, params: InitializeParams) -> Self {
        let root = params.root_path.clone().unwrap_or_default();
        Self {
            cache: Project::new(root),
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
            Completion(id, params) => {
                mlog!("Got completion request #{id}: {params:?}");
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

        let result = if let Some(doc) = self.cache.documents.get(&uri) {
            // Loop through the tokens and convert them
            let tokens = SemanticTokens {
                result_id: None,
                data: doc.get_semantic_tokens().data,
            };
            info!("Semantic Tokens for {uri} {tokens:?}");
            Some(SemanticTokensResult::Tokens(tokens))
        } else {
            None
        };

        return result;
    }

    fn handle_folding_range_request(
        &mut self,
        id: &RequestId,
        params: FoldingRangeParams,
    ) -> Option<Vec<FoldingRange>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.cache.documents.get(&uri) {
            if !doc.folding_ranges.is_empty() {
                return Some(doc.folding_ranges.clone());
            } else {
                return None;
            }
        } else {
            return None;
        }
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
        info!("Document Opened: {uri}");

        let document = self.cache.insert(&uri, &params.text_document.text);

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

        Ok(())
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

            let document = self.cache.insert(&uri, &change.text);

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
        }

        Ok(())
    }
}
