#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn lint(source: &str) -> String {
    let diags = crate::lint_source("input.qs", source);
    crate::output::json::format_json(&diags)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn lint_with_options(source: &str, include_ast: bool) -> String {
    let diags = crate::lint_source_with_options("input.qs", source, include_ast);
    crate::output::json::format_json(&diags)
}

pub fn lint_to_json(source: &str) -> String {
    let diags = crate::lint_source("input.qs", source);
    crate::output::json::format_json(&diags)
}

pub fn lint_to_json_with_options(source: &str, include_ast: bool) -> String {
    let diags = crate::lint_source_with_options("input.qs", source, include_ast);
    crate::output::json::format_json(&diags)
}

