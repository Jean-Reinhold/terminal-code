use serde::{Deserialize, Serialize};

pub type Rgb = [u8; 3];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalPalette {
    pub background: Rgb,
    pub foreground: Rgb,
    pub ansi: [Rgb; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedReplies {
    pub background: Option<Rgb>,
    pub foreground: Option<Rgb>,
    pub ansi: [Option<Rgb>; 16],
}

pub fn parse_color(reply: &str) -> Option<Rgb> {
    let lower = reply.to_ascii_lowercase();
    let start = lower.find("rgb:")? + 4;
    let mut components = lower[start..].split('/');
    let red = scale_component(components.next()?)?;
    let green = scale_component(components.next()?)?;
    let blue = scale_component(components.next()?)?;
    Some([red, green, blue])
}

pub fn parse_replies(raw: &str) -> ParsedReplies {
    let mut parsed = ParsedReplies {
        background: None,
        foreground: None,
        ansi: [None; 16],
    };
    let bytes = raw.as_bytes();
    let mut cursor = 0;
    while cursor + 2 < bytes.len() {
        let Some(relative) = bytes[cursor..]
            .windows(2)
            .position(|window| window == b"\x1b]")
        else {
            break;
        };
        let start = cursor + relative + 2;
        let mut end = start;
        while end < bytes.len() && bytes[end] != 0x07 {
            if bytes[end] == 0x1b && bytes.get(end + 1) == Some(&b'\\') {
                break;
            }
            end += 1;
        }
        if end >= bytes.len() {
            break;
        }
        if let Ok(payload) = std::str::from_utf8(&bytes[start..end]) {
            apply_reply(payload, &mut parsed);
        }
        cursor = if bytes[end] == 0x1b { end + 2 } else { end + 1 };
    }
    parsed
}

pub fn build_query() -> String {
    let mut query = String::from("\x1b]11;?\x07\x1b]10;?\x07");
    for index in 0..16 {
        query.push_str(&format!("\x1b]4;{index};?\x07"));
    }
    query.push_str("\x1b[c");
    query
}

pub fn with_fallbacks(parsed: Option<&ParsedReplies>) -> TerminalPalette {
    let ansi = std::array::from_fn(|index| {
        parsed
            .and_then(|value| value.ansi[index])
            .unwrap_or(FALLBACK_ANSI[index])
    });
    TerminalPalette {
        background: parsed
            .and_then(|value| value.background)
            .unwrap_or([13, 15, 19]),
        foreground: parsed
            .and_then(|value| value.foreground)
            .unwrap_or([230, 233, 239]),
        ansi,
    }
}

fn apply_reply(payload: &str, parsed: &mut ParsedReplies) {
    let mut fields = payload.splitn(3, ';');
    let Some(code) = fields.next() else { return };
    match code {
        "10" => {
            if let Some(body) = fields.next() {
                parsed.foreground = parse_color(body);
            }
        }
        "11" => {
            if let Some(body) = fields.next() {
                parsed.background = parse_color(body);
            }
        }
        "4" => {
            let (Some(index), Some(body)) = (fields.next(), fields.next()) else {
                return;
            };
            let Ok(index) = index.parse::<usize>() else {
                return;
            };
            if index < 16 {
                parsed.ansi[index] = parse_color(body);
            }
        }
        _ => {}
    }
}

fn scale_component(raw: &str) -> Option<u8> {
    let digits = raw.bytes().take_while(u8::is_ascii_hexdigit).count();
    if digits == 0 {
        return None;
    }
    let raw = &raw[..digits];
    let value = u128::from_str_radix(raw, 16).ok()?;
    let maximum = 16_u128.checked_pow(digits as u32)?.checked_sub(1)?;
    Some(((value as f64 / maximum as f64) * 255.0).round() as u8)
}

const FALLBACK_ANSI: [Rgb; 16] = [
    [26, 27, 30],
    [229, 72, 77],
    [48, 164, 108],
    [245, 165, 36],
    [93, 156, 255],
    [186, 148, 255],
    [94, 201, 227],
    [200, 205, 215],
    [90, 96, 106],
    [255, 108, 112],
    [76, 194, 138],
    [255, 196, 84],
    [124, 178, 255],
    [206, 176, 255],
    [126, 220, 240],
    [235, 238, 245],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_whatever_component_width_the_terminal_uses() {
        assert_eq!(parse_color("rgb:0000/0000/0000"), Some([0, 0, 0]));
        assert_eq!(parse_color("rgb:ffff/ffff/ffff"), Some([255, 255, 255]));
        assert_eq!(parse_color("rgb:ff/80/00"), Some([255, 128, 0]));
        assert_eq!(parse_color("no colour"), None);
    }

    #[test]
    fn parses_background_foreground_and_ansi_replies() {
        let raw = "\x1b]11;rgb:0000/0000/0000\x07\x1b]10;rgb:c8c8/cdcd/d7d7\x1b\\\x1b]4;1;rgb:ffff/0000/0000\x07\x1b]4;12;rgb:1111/2222/3333\x1b\\";
        let parsed = parse_replies(raw);
        assert_eq!(parsed.background, Some([0, 0, 0]));
        assert_eq!(parsed.foreground, Some([200, 205, 215]));
        assert_eq!(parsed.ansi[1], Some([255, 0, 0]));
        assert_eq!(parsed.ansi[12], Some([17, 34, 51]));
    }

    #[test]
    fn fills_only_missing_palette_values() {
        let mut parsed = ParsedReplies {
            background: Some([0, 0, 0]),
            foreground: None,
            ansi: [None; 16],
        };
        parsed.ansi[1] = Some([1, 2, 3]);
        let palette = with_fallbacks(Some(&parsed));
        assert_eq!(palette.background, [0, 0, 0]);
        assert_eq!(palette.foreground, [230, 233, 239]);
        assert_eq!(palette.ansi[1], [1, 2, 3]);
        assert_eq!(palette.ansi[2], [48, 164, 108]);
        assert_eq!(palette.ansi.len(), 16);
    }

    #[test]
    fn query_ends_with_device_attributes_after_all_colours() {
        let query = build_query();
        assert!(query.starts_with("\x1b]11;?\x07\x1b]10;?\x07"));
        assert!(query.contains("\x1b]4;15;?\x07"));
        assert!(query.ends_with("\x1b[c"));
    }
}
