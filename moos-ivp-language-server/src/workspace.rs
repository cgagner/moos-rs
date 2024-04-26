// TODO: This is just a test to see how long it would take to scan
// a large repository

use lsp_server::{Connection, Message};
use lsp_types::{PublishDiagnosticsParams, Url};
use moos_parser::PlugParser;
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tracing::info;

use crate::cache::Project;

struct PathUrl {
    path: PathBuf,
    url: Url,
}
fn is_moos_workspace_file(filename: &str, extension: &str) -> bool {
    let extension = extension.to_ascii_lowercase();
    match extension.as_str() {
        "moos" | "moos++" | "bhv" | "bhv++" | "plug" | "def" => return true,
        // TODO: We should check for launch scripts
        //"bash" | "sh" | "zsh" => return Self::Script,
        "mfs" | "gfs" => return true,
        _ => {}
    }

    let filename = filename.to_ascii_lowercase();
    if filename.starts_with("plug_")
        || filename.starts_with("meta_")
        || filename.starts_with("app_")
        || filename.starts_with("moos_")
        || filename.starts_with("bhv_")
    {
        return true;
    }

    return false;
}
fn visit_file(path: &Path, root_path: &str, root_uri: &Url) -> Option<PathUrl> {
    if !path.is_file() {
        return None;
    }

    if let Some(extension) = path.extension() {
        if let Some(extension) = extension.to_str() {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            if !is_moos_workspace_file(filename, extension) {
                return None;
            }

            if let Ok(relative_path) = path.strip_prefix(Path::new(root_path)) {
                if let Some(rel_path) = relative_path.to_str() {
                    if let Ok(rel_uri) = root_uri.join(rel_path) {
                        return Some(PathUrl {
                            path: path.to_owned(),
                            url: rel_uri,
                        });
                    }
                }
            }
        }
    }

    return None;
}

fn visit_dirs(dir: &Path, root_path: &str, root_uri: &Url) -> Vec<PathUrl> {
    if let Ok(dir) = fs::read_dir(dir) {
        return dir
            .into_iter()
            .filter_map(|f| f.ok())
            .map(|entry| -> Vec<PathUrl> {
                //
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, root_path, root_uri)
                } else {
                    if let Some(path_url) = visit_file(&path, root_path, root_uri) {
                        vec![path_url]
                    } else {
                        Vec::new()
                    }
                }
            })
            .flatten()
            .collect();
    }

    return Vec::new();
}

// TODO: Add method for finding a file in a workspace

pub fn scan_workspace(
    connection: &Connection,
    cache: Arc<Mutex<Project>>,
    root_path: String,
    root_uri: Url,
) -> anyhow::Result<()> {
    let added = visit_dirs(Path::new(&root_path), root_path.as_str(), &root_uri);

    let sender = connection.sender.clone();

    // TODO: Spawning this thread does load the files, but that is pretty
    // useless at the moment. We need a way to get the client to send us the
    // information about the workspace so we can send diagnostics.

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2000));

        let start = Instant::now();

        for path_url in added {
            if let Ok(text) = fs::read_to_string(path_url.path) {
                if let Ok(cache) = &mut cache.lock() {
                    let document = cache.insert(&path_url.url, text);

                    // TODO: Send diagnostics - Sending the diagnostics using
                    // the method below does not seem to work for VSCode.
                    // There are some mentions that the LSP protocol only
                    // allows sending diagnostics for opened buffers.

                    // let diagnostics = PublishDiagnosticsParams::new(
                    //     path_url.url.clone(),
                    //     document.diagnostics.clone(),
                    //     None,
                    // );

                    // let params = serde_json::to_value(&diagnostics).unwrap();
                    // use lsp_types::notification::Notification;
                    // let notification = lsp_server::Notification {
                    //     method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                    //     params,
                    // };

                    // if let Ok(r) = sender.send(Message::Notification(notification)) {
                    // } else {
                    //     tracing::error!("Failed to send notification: {:?}", root_path);
                    // }
                }
            }
        }

        let diff = (Instant::now() - start).as_millis() as f64 * 1e-3;
        tracing::info!("Workspace: parse time: {diff}")
    });

    Ok(())
}
