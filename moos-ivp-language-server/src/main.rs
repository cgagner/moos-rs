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

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    notification::DidChangeTextDocument, notification::Notification, request::GotoDefinition,
    GotoDefinitionResponse, InitializeParams, ServerCapabilities, TextDocumentSyncCapability,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, OneOf, TextDocumentSyncKind,
};

use tracing::debug as mlog;
use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

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
                match cast_request::<GotoDefinition>(req) {
                    Ok((id, params)) => {
                        mlog!("got gotoDefinition request #{id}: {params:?}");
                        let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                        let result = serde_json::to_value(&result).unwrap();
                        let resp = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;
                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };
                // ...
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

fn cast_request<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn handle_notification(notification: lsp_server::Notification) -> anyhow::Result<()> {
    use serde_json::from_value;
    let method = notification.method.as_str();
    let params = notification.params;
    match method {
        DidChangeTextDocument::METHOD => {
            let change_params: DidChangeTextDocumentParams = from_value(params)?;
            debug!("Handle change: {change_params:?}");
        }
        _ => info!("Unexpected notification: {method}"),
    };

    Ok(())
}
