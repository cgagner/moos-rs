/*
 * Parsers:
 *  - [ ] NSPlug
 *  - [ ] MOOS Missions
 *  - [ ] IvP Behavior files
 *
 * Desired Language Features:
 *  - [ ] Parse Workspace Configuration
 *  - [ ] Semantic Tokens
 *  - [ ] Diagnostics
 *  - [ ] Code Actions
 *  - [ ] Go to Definitions
 *  - [ ] Document links
 *  - [ ] Completion Suggestions
 *  - [ ] Hover
 *    - [ ] Show values of variables
 *    - [ ] Show help text for Mission ProcessConfig parameters
 *    - [ ] Show help text for Behavior parameters
 *  - [ ] Format
 *    - [ ] Plug files
 *    - [ ] Mission files
 *    - [ ] Behavior files
 *  - [ ] Tracing via setTrace
 *
 * TODO:
 *  - [x] Setup tracing
 *  - [ ] Handle command line arguments
 *  - [ ] Create cache of files being changed. Need to clear cache when files
 *        are saved or closed.
 *  - [ ] Create Threads for parsing the entire workspace
 *  - [ ] Create thread for formatting
 *
 *  Wishlist:
 *  - [ ] Implement fuzzy find
 *  - [ ] Implement Levenshtein distance for closest word suggestions
 */

// Make clippy print to standard error so it doesn't interfere with
// the lsp interactions with stdio.
#![allow(clippy::print_stderr)]

mod cache;
mod tracer;

use std::error::Error;

// NOTE: Do not use lsp_server::{Notification, Request}. These conflict with
// other types.

use lsp_server::{Connection, ExtractError, Message, RequestId, Response};
use lsp_types::{
    notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidOpenTextDocument, Notification,
    },
    request::{GotoDefinition, Request},
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
}

// Declare the Notifications we are going to handle.
#[notification_handler]
enum MyNotifications {
    DidChangeTextDocument,
    DidOpenTextDocument,
    DidChangeConfiguration,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    tracer::Tracer::init()?;

    // Note that  we must have our logging only write out to stderr.
    info!("Starting MOOS-IvP LSP server");

    // Create the connection to the client.
    // TODO: Need to add support for --port and --pipe
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    })?;

    // TODO: Handle client capabilities

    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    info!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params)?;
    info!("starting example main loop");
    for msg in &connection.receiver {
        trace!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                mlog!("got request: {req:?}");
                use MyRequests::*;
                match MyRequests::from(req) {
                    GotoDefinition(id, params) => {
                        mlog!("got gotoDefinition request #{id}: {params:?}");
                        let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                        let result = serde_json::to_value(&result).unwrap();
                        let resp = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;
                    }
                    Unhandled(req) => info!("Unhandled Request {:?}", req.method),
                    Error { method, error } => {
                        error!("Failed to handle Request {method}: {error:?}")
                    }
                }
            }
            Message::Response(resp) => {
                mlog!("got response: {resp:?}");
            }
            Message::Notification(notification) => {
                mlog!("got notification: {notification:?}");
                if let Err(e) = handle_notification(notification) {
                    error!("Failed to handle notification: {e:?}");
                }
            }
        }
    }
    Ok(())
}

fn handle_notification(notification: lsp_server::Notification) -> anyhow::Result<()> {
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
