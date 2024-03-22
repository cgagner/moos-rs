use std::error::Error;

use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{
    notification::{DidChangeConfiguration, DidChangeTextDocument, DidOpenTextDocument},
    request::{Completion, GotoDefinition},
    GotoDefinitionResponse, InitializeParams, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};

use tracing::debug as mlog;
use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

use lsp_server_derive_macro::{notification_handler, request_handler};

// Declare the Requests that we are going to handle.
#[request_handler]
enum MyRequests {
    GotoDefinition,
    Completion,
}

// Declare the Notifications we are going to handle.
#[notification_handler]
enum MyNotifications {
    DidChangeTextDocument,
    DidOpenTextDocument,
    DidChangeConfiguration,
}

//-----------------------------------------------------------------------------
// Requests
//-----------------------------------------------------------------------------
pub fn handle_request(request: lsp_server::Request) -> anyhow::Result<()> {
    use MyRequests::*;
    info!("Got request: {:?}", request.method);
    match MyRequests::from(request) {
        GotoDefinition(id, params) => {
            mlog!("got gotoDefinition request #{id}: {params:?}");
            let result = Some(GotoDefinitionResponse::Array(Vec::new()));
            let result = serde_json::to_value(&result).unwrap();
            let resp = Response {
                id,
                result: Some(result),
                error: None,
            };
            //connection.sender.send(Message::Response(resp))?;
        }
        Completion(id, params) => {
            mlog!("Got completion request #{id}: {params:?}");
        }
        Unhandled(req) => info!("Unhandled Request {:?}", req.method),
        Error { method, error } => {
            error!("Failed to handle Request {method}: {error:?}")
        }
    }
    Ok(())
}
//-----------------------------------------------------------------------------
// Notifications
//-----------------------------------------------------------------------------
pub fn handle_notification(notification: lsp_server::Notification) -> anyhow::Result<()> {
    use MyNotifications::*;
    match MyNotifications::from(notification) {
        DidChangeConfiguration(params) => {
            info!("Configuration Changed: {params:?}")
        }
        DidChangeTextDocument(params) => info!("Document Changed: {params:?}"),
        DidOpenTextDocument(params) => info!("Document Opened: {params:?}"),
        Unhandled(n) => info!("Unhandled Notification: {:?}", n.method),
        Error { method, error } => error!("Failed to handle Notification {method}: {error:?}"),
    }

    Ok(())
}
