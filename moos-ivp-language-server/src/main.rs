/*
 * Parsers:
 *  - [ ] NSPlug
 *  - [ ] MOOS Missions
 *  - [ ] IvP Behavior files
 *  - [ ] MOOS-IvP Emacs settings for keywords
 *  - [ ] Create new file format for Behavior and MOOS app variables with description
 *  - [ ] MOOS-IvP Manifests
 *  - [ ] Create method for converting between Parser Range and LSP Range.
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
 *    - [ ] Left justified, left justified aligned equals, right justified
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
mod handle;
mod lsp;
mod tracer;
mod trees;

use std::error::Error;

use lsp_server::{Connection, Message, RequestId};
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

use crate::handle::{handle_notification, handle_request};

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
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                handle_request(request);
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
