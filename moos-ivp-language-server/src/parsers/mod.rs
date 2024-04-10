use lsp_types::Url;

pub mod moos;
pub mod nsplug;

/// Find a file relative to a parent URL.
///
/// TODO: This should really search the workspace, but for now we will
/// assume this can search the local file system.
fn find_relative_file(parent_url: &Url, file_name: &str) -> Option<Url> {
    if parent_url.scheme() == "file" {
        let parent_path = std::path::Path::new(parent_url.path());
        if parent_path.exists() && parent_path.is_file() {
            if let Some(parent_dir) = parent_path.parent() {
                let include_path = parent_dir.join(file_name.to_string());
                if include_path.exists() && include_path.is_file() {
                    let mut new_url = parent_url.clone();
                    if let Some(path_str) = include_path.to_str() {
                        new_url.set_path(path_str);
                        return Some(new_url);
                    }
                }
            }
        }
    }

    return None;
}
