use std::collections::HashMap;

use lsp_types::{SemanticTokens, Url};

struct Project {}

struct Document {
    uri: Url,
    text: String,
    semantic_tokens: SemanticTokens,
}

struct Delcaration {}

struct Definition {}

struct CacheData {
    document: Url,
    text: String,
}

type Cache = HashMap<Url, CacheData>;
