use crate::error::{KratosError, KratosResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsoncDocument {
    pub raw: String,
}

pub fn strip_jsonc_comments(source: &str) -> KratosResult<String> {
    let _ = source;
    Err(KratosError::not_implemented(
        "jsonc::strip_jsonc_comments",
    ))
}

pub fn parse_jsonc_document(source: &str) -> KratosResult<JsoncDocument> {
    let _ = source;
    Err(KratosError::not_implemented(
        "jsonc::parse_jsonc_document",
    ))
}

pub fn parse_loose_json(_source: &str) -> KratosResult<JsoncDocument> {
    Err(KratosError::not_implemented("jsonc::parse_loose_json"))
}
