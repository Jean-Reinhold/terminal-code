use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Target {
    pub folder: Option<PathBuf>,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFile {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
}

pub fn resolve_target(argument: Option<&str>, cwd: &Path) -> Target {
    let requested = absolute_from(cwd, argument.unwrap_or("."));
    match fs::metadata(&requested) {
        Ok(metadata) if metadata.is_dir() => Target {
            folder: Some(requested),
            file: None,
        },
        _ => Target {
            folder: None,
            file: Some(requested),
        },
    }
}

pub fn parse_goto(argument: &str, cwd: &Path) -> OpenFile {
    let candidate = Path::new(argument);
    let existing = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        cwd.join(candidate)
    };
    if existing.exists() {
        return OpenFile {
            path: argument.to_owned(),
            line: None,
            column: None,
        };
    }

    let Some((before_last, last)) = argument.rsplit_once(':') else {
        return plain_file(argument);
    };
    let Ok(last_number) = last.parse::<u64>() else {
        return plain_file(argument);
    };
    if let Some((path, line)) = before_last.rsplit_once(':')
        && let Ok(line_number) = line.parse::<u64>()
    {
        return OpenFile {
            path: path.to_owned(),
            line: Some(line_number),
            column: Some(last_number),
        };
    }
    OpenFile {
        path: before_last.to_owned(),
        line: Some(last_number),
        column: Some(1),
    }
}

fn absolute_from(cwd: &Path, argument: &str) -> PathBuf {
    let path = Path::new(argument);
    if path.is_absolute() {
        normalize_lexically(path)
    } else {
        normalize_lexically(&cwd.join(path))
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn plain_file(argument: &str) -> OpenFile {
    OpenFile {
        path: argument.to_owned(),
        line: None,
        column: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn resolves_existing_and_missing_targets() {
        let root = TempDir::new().unwrap();
        let file = root.path().join("present.txt");
        let folder = root.path().join("folder");
        fs::write(&file, "present").unwrap();
        fs::create_dir(&folder).unwrap();

        assert_eq!(
            resolve_target(Some("present.txt"), root.path()),
            Target {
                folder: None,
                file: Some(file)
            }
        );
        assert_eq!(
            resolve_target(Some("folder"), root.path()),
            Target {
                folder: Some(folder),
                file: None
            }
        );
        assert_eq!(
            resolve_target(Some("missing.txt"), root.path()),
            Target {
                folder: None,
                file: Some(root.path().join("missing.txt"))
            }
        );
    }

    #[test]
    fn parses_goto_and_preserves_existing_numeric_suffix() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("existing:12"), "present").unwrap();

        assert_eq!(
            parse_goto("src/main.ts:12:4", root.path()),
            OpenFile {
                path: "src/main.ts".into(),
                line: Some(12),
                column: Some(4)
            }
        );
        assert_eq!(
            parse_goto("src/main.ts:12", root.path()),
            OpenFile {
                path: "src/main.ts".into(),
                line: Some(12),
                column: Some(1)
            }
        );
        assert_eq!(
            parse_goto("existing:12", root.path()),
            OpenFile {
                path: "existing:12".into(),
                line: None,
                column: None
            }
        );
    }
}
