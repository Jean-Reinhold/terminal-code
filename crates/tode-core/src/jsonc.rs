use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn set_key(source: &str, key: &str, value: &Value) -> String {
    let encoded = serde_json::to_string(value).expect("serde_json::Value always serializes");
    let text = if source.trim().is_empty() {
        "{}"
    } else {
        source
    };
    if let Some((start, end)) = locate(text, key) {
        return format!("{}{}{}", &text[..start], encoded, &text[end..]);
    }
    let Some(brace) = root_brace(text) else {
        let key = serde_json::to_string(key).expect("string key serializes");
        return format!("{{\n  {key}: {encoded}\n}}\n");
    };
    let indent = indent_of(text, brace + 1);
    let empty = text.as_bytes().get(skip_trivia(text, brace + 1)) == Some(&b'}');
    let key = serde_json::to_string(key).expect("string key serializes");
    let line = format!("\n{indent}{key}: {encoded}{}", if empty { "" } else { "," });
    let rest = &text[brace + 1..];
    let gap = if empty && !rest.starts_with('\n') {
        "\n"
    } else {
        ""
    };
    format!("{}{}{}{}", &text[..brace + 1], line, gap, rest)
}

pub fn set_keys(source: &str, entries: &BTreeMap<String, Value>) -> String {
    let mut text = source.to_owned();
    for (key, value) in entries {
        text = set_key(&text, key, value);
    }
    text
}

pub fn parse_jsonc<T: DeserializeOwned>(source: &str) -> Option<T> {
    let mut stripped = String::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        if starts_with(source, index, b"//") {
            index = skip_line_comment(source, index);
            continue;
        }
        if starts_with(source, index, b"/*") {
            index = skip_block_comment(source, index);
            continue;
        }
        if source.as_bytes()[index] == b'"' {
            let end = read_string(source, index).1;
            stripped.push_str(&source[index..end]);
            index = end;
            continue;
        }
        let character = source[index..].chars().next()?;
        stripped.push(character);
        index += character.len_utf8();
    }
    let without_trailing = remove_trailing_commas(&stripped);
    serde_json::from_str(&without_trailing).ok()
}

pub fn read_key(source: &str, key: &str) -> Option<Value> {
    let (start, end) = locate(source, key)?;
    serde_json::from_str(&source[start..end]).ok()
}

fn skip_line_comment(source: &str, at: usize) -> usize {
    source[at..]
        .find('\n')
        .map(|offset| at + offset)
        .unwrap_or(source.len())
}

fn skip_block_comment(source: &str, at: usize) -> usize {
    source[at + 2..]
        .find("*/")
        .map(|offset| at + 2 + offset + 2)
        .unwrap_or(source.len())
}

fn read_string(source: &str, at: usize) -> (String, usize) {
    let mut index = at + 1;
    let mut value = String::new();
    while index < source.len() {
        let byte = source.as_bytes()[index];
        if byte == b'\\' {
            if let Some(character) = source[index + 1..].chars().next() {
                value.push(character);
                index += 1 + character.len_utf8();
            } else {
                index += 1;
            }
            continue;
        }
        if byte == b'"' {
            return (value, index + 1);
        }
        let Some(character) = source[index..].chars().next() else {
            break;
        };
        value.push(character);
        index += character.len_utf8();
    }
    (value, index)
}

fn skip_trivia(source: &str, at: usize) -> usize {
    let mut index = at;
    while index < source.len() {
        if starts_with(source, index, b"//") {
            index = skip_line_comment(source, index);
            continue;
        }
        if starts_with(source, index, b"/*") {
            index = skip_block_comment(source, index);
            continue;
        }
        let Some(character) = source[index..].chars().next() else {
            break;
        };
        if !character.is_whitespace() {
            return index;
        }
        index += character.len_utf8();
    }
    index
}

fn container_end(source: &str, at: usize) -> usize {
    let mut index = at;
    let mut depth = 0_i32;
    while index < source.len() {
        if starts_with(source, index, b"//") {
            index = skip_line_comment(source, index);
            continue;
        }
        if starts_with(source, index, b"/*") {
            index = skip_block_comment(source, index);
            continue;
        }
        match source.as_bytes()[index] {
            b'"' => {
                index = read_string(source, index).1;
                continue;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += source[index..].chars().next().map_or(1, char::len_utf8);
    }
    index
}

fn value_end(source: &str, at: usize) -> usize {
    match source.as_bytes().get(at) {
        Some(b'"') => return read_string(source, at).1,
        Some(b'{') | Some(b'[') => return container_end(source, at),
        _ => {}
    }
    let mut index = at;
    while index < source.len() {
        let Some(character) = source[index..].chars().next() else {
            break;
        };
        if character.is_whitespace() || ",}]".contains(character) {
            break;
        }
        if starts_with(source, index, b"//") || starts_with(source, index, b"/*") {
            break;
        }
        index += character.len_utf8();
    }
    index
}

fn locate(source: &str, key: &str) -> Option<(usize, usize)> {
    let mut index = 0;
    let mut depth = 0_i32;
    let mut found = None;
    while index < source.len() {
        if starts_with(source, index, b"//") {
            index = skip_line_comment(source, index);
            continue;
        }
        if starts_with(source, index, b"/*") {
            index = skip_block_comment(source, index);
            continue;
        }
        match source.as_bytes()[index] {
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                index += 1;
            }
            b'"' => {
                let (literal, end) = read_string(source, index);
                if depth == 1 && literal == key {
                    let colon = skip_trivia(source, end);
                    if source.as_bytes().get(colon) == Some(&b':') {
                        let start = skip_trivia(source, colon + 1);
                        found = Some((start, value_end(source, start)));
                    }
                }
                index = end;
            }
            _ => index += source[index..].chars().next().map_or(1, char::len_utf8),
        }
    }
    found
}

fn root_brace(source: &str) -> Option<usize> {
    let mut index = 0;
    while index < source.len() {
        if starts_with(source, index, b"//") {
            index = skip_line_comment(source, index);
            continue;
        }
        if starts_with(source, index, b"/*") {
            index = skip_block_comment(source, index);
            continue;
        }
        if source.as_bytes()[index] == b'{' {
            return Some(index);
        }
        index += source[index..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

fn indent_of(source: &str, after_brace: usize) -> String {
    let rest = &source[after_brace..];
    let mut lines = rest.split('\n');
    lines.next();
    for line in lines {
        let indent: String = line
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .collect();
        if !line[indent.len()..].trim().is_empty() && !indent.is_empty() {
            return indent;
        }
    }
    "  ".into()
}

fn remove_trailing_commas(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        if source.as_bytes()[index] == b'"' {
            let end = read_string(source, index).1;
            output.push_str(&source[index..end]);
            index = end;
            continue;
        }
        if source.as_bytes()[index] == b',' {
            let mut next = index + 1;
            while next < source.len() {
                let character = source[next..].chars().next().unwrap();
                if !character.is_whitespace() {
                    break;
                }
                next += character.len_utf8();
            }
            if matches!(source.as_bytes().get(next), Some(b'}' | b']')) {
                index += 1;
                continue;
            }
        }
        let character = source[index..].chars().next().unwrap();
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn starts_with(source: &str, at: usize, needle: &[u8]) -> bool {
    source.as_bytes().get(at..at + needle.len()) == Some(needle)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn edits_keep_comments_and_unrelated_keys() {
        let before =
            "{\n  // a note\n  \"editor.tabSize\": 4,\n  \"workbench.colorTheme\": \"Old\",\n}\n";
        let after = set_key(before, "workbench.colorTheme", &json!("Terminal Code"));
        assert!(after.contains("// a note"));
        assert!(after.contains("\"editor.tabSize\": 4"));
        assert!(after.contains("\"workbench.colorTheme\": \"Terminal Code\""));
    }

    #[test]
    fn writes_empty_absent_and_non_object_sources() {
        assert_eq!(read_key(&set_key("", "a", &json!(1)), "a"), Some(json!(1)));
        assert_eq!(
            read_key(&set_key("{}", "a", &json!("x")), "a"),
            Some(json!("x"))
        );
        assert_eq!(set_key("not json", "a", &json!(1)), "{\n  \"a\": 1\n}\n");
    }

    #[test]
    fn repeated_writes_are_byte_stable() {
        let once = set_key("{}", "a", &json!({"nested": [1, 2]}));
        let twice = set_key(&once, "a", &json!({"nested": [1, 2]}));
        assert_eq!(once, twice);
    }

    #[test]
    fn parses_comments_trailing_commas_and_arrays() {
        let object: Value =
            parse_jsonc("{ // line\n \"a\": 1, /* block */ \"nested\": {\"b\": 2,}, }").unwrap();
        assert_eq!(object, json!({"a": 1, "nested": {"b": 2}}));
        let array: Value = parse_jsonc("// mine\n[ { \"key\": \"cmd+k\" }, ]").unwrap();
        assert_eq!(array, json!([{"key": "cmd+k"}]));
        assert!(parse_jsonc::<Value>("{ this is not json").is_none());
    }

    #[test]
    fn only_root_keys_are_located() {
        let source = "{\"a\":1,\"nested\":{\"a\":2}}";
        assert_eq!(read_key(source, "a"), Some(json!(1)));
        assert_eq!(
            set_key(source, "a", &json!(3)),
            "{\"a\":3,\"nested\":{\"a\":2}}"
        );
    }
}
