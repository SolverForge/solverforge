/// Extracts `SolutionType` and `ScoreType` from `impl ConstraintSet<S, T>` in mod.rs.
/// Falls back to `Plan` / `HardSoftScore` on failure.
pub(crate) fn extract_types(src: &str) -> (String, String) {
    if let Some(pos) = src.find("ConstraintSet<") {
        let after = &src[pos + "ConstraintSet<".len()..];
        if let Some(end) = after.find('>') {
            let inner = &after[..end];
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            if parts.len() == 2 {
                let s = parts[0].trim().to_string();
                let t = parts[1].trim().to_string();
                if !s.is_empty() && !t.is_empty() {
                    return (s, t);
                }
            }
        }
    }
    eprintln!("warning: could not extract types from mod.rs; falling back to Plan / HardSoftScore");
    ("Plan".to_string(), "HardSoftScore".to_string())
}

/// Rewrites `src/constraints/mod.rs` to declare the new module and extend the tuple.
pub(crate) fn rewrite_mod(src: &str, name: &str) -> String {
    let mod_decl = format!("mod {};", name);
    let call = format!("{}::constraint()", name);

    if src.contains("mod assemble {") {
        rewrite_assemble_shape(src, &mod_decl, &call)
    } else {
        rewrite_flat_shape(src, &mod_decl, &call)
    }
}

fn rewrite_assemble_shape(src: &str, mod_decl: &str, call: &str) -> String {
    let src = insert_mod_decl_assemble(src, mod_decl);
    extend_tuple(&src, call)
}

fn rewrite_flat_shape(src: &str, mod_decl: &str, call: &str) -> String {
    let src = insert_mod_decl_flat(src, mod_decl);
    extend_tuple(&src, call)
}

pub(crate) fn insert_mod_decl_assemble(src: &str, mod_decl: &str) -> String {
    let mut last_mod_end = None;
    for line in src.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
            let pos = find_line_end_pos(src, line);
            last_mod_end = Some(pos);
        }
    }

    if let Some(pos) = last_mod_end {
        let mut result = src.to_string();
        result.insert_str(pos, &format!("\n{}", mod_decl));
        result
    } else {
        format!("{}\n{}", mod_decl, src)
    }
}

fn insert_mod_decl_flat(src: &str, mod_decl: &str) -> String {
    let mut insert_pos = 0;
    for line in src.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("//!") || trimmed.is_empty() {
            insert_pos += line.len();
        } else {
            break;
        }
    }

    let mut result = src.to_string();
    if insert_pos > 0 {
        result.insert_str(insert_pos, &format!("\n{}\n", mod_decl));
    } else {
        result.insert_str(0, &format!("{}\n\n", mod_decl));
    }
    result
}

pub(crate) fn extend_tuple(src: &str, call: &str) -> String {
    let fn_start = match src.find("fn create_constraints") {
        Some(p) => p,
        None => return src.to_string(),
    };

    let body_start = match src[fn_start..].find('{') {
        Some(p) => fn_start + p + 1,
        None => return src.to_string(),
    };

    let mut depth = 1usize;
    let body_chars: Vec<char> = src[body_start..].chars().collect();
    let mut byte_offset = body_start;
    for ch in &body_chars {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        byte_offset += ch.len_utf8();
    }
    let body_end = byte_offset;

    let body = &src[body_start..body_end];
    let last_paren = match body.rfind(')') {
        Some(p) => body_start + p,
        None => return src.to_string(),
    };

    let before = &src[..last_paren];
    let after = &src[last_paren..];

    let trimmed_before = before.trim_end();
    if trimmed_before.ends_with(',') {
        format!("{} {}{}", before, call, after)
    } else {
        format!("{}, {}{}", before, call, after)
    }
}

fn find_line_end_pos(src: &str, line: &str) -> usize {
    let line_ptr = line.as_ptr() as usize;
    let src_ptr = src.as_ptr() as usize;
    let offset = line_ptr - src_ptr;
    offset + line.len()
}
