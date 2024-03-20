use std::collections::HashMap;

use lsp_types::Url;

struct CacheData {
    document: Url,
    text: String,
}

type Cache = HashMap<Url, CacheData>;
