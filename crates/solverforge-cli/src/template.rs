use include_dir::{Dir, DirEntry};
use std::fs;
use std::path::Path;

use crate::error::{CliError, CliResult};

// Looks for a custom template override in `.solverforge/templates/<template_name>.rs.tmpl`.
// Template variables use `{{NAME}}`, `{{SNAKE_NAME}}`, `{{FIELDS}}` style placeholders.
// Returns the rendered content if a custom template exists, or `None` to use the built-in default.
pub fn load_custom(template_name: &str, vars: &[(&str, &str)]) -> Option<String> {
    let path = Path::new(".solverforge")
        .join("templates")
        .join(format!("{}.rs.tmpl", template_name));

    let contents = fs::read_to_string(&path).ok()?;
    Some(apply_vars(&contents, vars))
}

/// Renders a template directory into `dest`, replacing `{{key}}` placeholders.
pub fn render(dir: &Dir, dest: &Path, vars: &[(&str, &str)]) -> CliResult {
    render_dir(dir, dest, vars)
}

fn render_dir(dir: &Dir, dest: &Path, vars: &[(&str, &str)]) -> CliResult {
    fs::create_dir_all(dest).map_err(|e| CliError::IoError {
        context: format!("failed to create directory {:?}", dest),
        source: e,
    })?;

    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => {
                let sub_name = sub
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let sub_dest = dest.join(&sub_name);
                render_dir(sub, &sub_dest, vars)?;
            }
            DirEntry::File(file) => {
                let file_name_raw = file
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                // Strip .tmpl extension if present
                let file_name = if file_name_raw.ends_with(".tmpl") {
                    file_name_raw[..file_name_raw.len() - 5].to_string()
                } else {
                    file_name_raw
                };

                let out_path = dest.join(&file_name);

                let contents = file.contents();

                // Binary files (non-UTF-8): copy as-is
                let Ok(text) = std::str::from_utf8(contents) else {
                    fs::write(&out_path, contents).map_err(|e| CliError::IoError {
                        context: format!("failed to write {:?}", out_path),
                        source: e,
                    })?;
                    continue;
                };

                // Text files: apply placeholder substitution
                let rendered = apply_vars(text, vars);
                fs::write(&out_path, rendered).map_err(|e| CliError::IoError {
                    context: format!("failed to write {:?}", out_path),
                    source: e,
                })?;
            }
        }
    }

    Ok(())
}

fn apply_vars(text: &str, vars: &[(&str, &str)]) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
