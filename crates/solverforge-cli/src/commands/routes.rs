use std::fs;
use std::path::Path;

use crate::error::{CliError, CliResult};
use crate::output;

// Parses src/api/ for axum `.route()` calls and prints a METHOD/PATH/HANDLER table.
pub fn run() -> CliResult {
    let candidates = ["src/api/routes.rs", "src/api/mod.rs", "src/api.rs"];

    let (source_path, content) = candidates
        .iter()
        .find_map(|p| {
            let path = Path::new(p);
            if path.exists() {
                fs::read_to_string(path).ok().map(|c| (*p, c))
            } else {
                None
            }
        })
        .ok_or(CliError::NotInProject {
            missing: "src/api/routes.rs or src/api/mod.rs",
        })?;

    output::print_status("routes", &format!("parsing {}", source_path));
    println!();

    let routes = parse_routes(&content);

    if routes.is_empty() {
        output::print_dim("  No routes found.");
        return Ok(());
    }

    // Column widths
    let method_w = routes
        .iter()
        .map(|(m, _, _)| m.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let path_w = routes
        .iter()
        .map(|(_, p, _)| p.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let handler_w = routes
        .iter()
        .map(|(_, _, h)| h.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "  {:<method_w$}  {:<path_w$}  {:<handler_w$}",
        "METHOD",
        "PATH",
        "HANDLER",
        method_w = method_w,
        path_w = path_w,
        handler_w = handler_w,
    );
    println!(
        "  {:-<method_w$}  {:-<path_w$}  {:-<handler_w$}",
        "",
        "",
        "",
        method_w = method_w,
        path_w = path_w,
        handler_w = handler_w,
    );

    for (method, path, handler) in &routes {
        println!(
            "  {:<method_w$}  {:<path_w$}  {:<handler_w$}",
            method,
            path,
            handler,
            method_w = method_w,
            path_w = path_w,
            handler_w = handler_w,
        );
    }
    println!();

    Ok(())
}

// Extracts (METHOD, PATH, HANDLER) triples from axum .route() calls.
fn parse_routes(content: &str) -> Vec<(String, String, String)> {
    let mut routes = Vec::new();

    // Match patterns like: .route("/path", get(handler)) or .route("/path", post(handler))
    let http_methods = [
        "get", "post", "put", "delete", "patch", "head", "options", "trace",
    ];

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.contains(".route(") {
            continue;
        }

        // Extract path string
        let path = extract_string_arg(trimmed).unwrap_or_default();

        // Extract method and handler
        for method in &http_methods {
            let pattern = format!("{}(", method);
            if let Some(pos) = trimmed.find(&pattern) {
                let after = &trimmed[pos + pattern.len()..];
                let handler = after
                    .split([')', ',', ' '])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !handler.is_empty() {
                    routes.push((method.to_uppercase(), path.clone(), handler));
                }
                break;
            }
        }
    }

    routes
}

// Extract the first string literal (double-quoted) from a line.
fn extract_string_arg(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')?;
    Some(line[start..start + end].to_string())
}
