use crate::Rgb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

pub fn to_oklch([r, g, b]: Rgb) -> Oklch {
    let lr = to_linear(r);
    let lg = to_linear(g);
    let lb = to_linear(b);
    let l = (0.412_221_470_8 * lr + 0.536_332_536_3 * lg + 0.051_445_992_9 * lb).cbrt();
    let m = (0.211_903_498_2 * lr + 0.680_699_545_1 * lg + 0.107_396_956_6 * lb).cbrt();
    let s = (0.088_302_461_9 * lr + 0.281_718_837_6 * lg + 0.629_978_700_5 * lb).cbrt();
    let lightness = 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s;
    let a = 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s;
    let bb = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s;
    Oklch {
        l: lightness,
        c: a.hypot(bb),
        h: bb.atan2(a),
    }
}

pub fn from_oklch(Oklch { l, c, h }: Oklch) -> Rgb {
    let a = h.cos() * c;
    let b = h.sin() * c;
    let lc = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let mc = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let sc = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        from_linear(4.076_741_662_1 * lc - 3.307_711_591_3 * mc + 0.230_969_929_2 * sc),
        from_linear(-1.268_438_004_6 * lc + 2.609_757_401_1 * mc - 0.341_319_396_5 * sc),
        from_linear(-0.004_196_086_3 * lc - 0.703_418_614_7 * mc + 1.707_614_701 * sc),
    ]
}

pub fn hex(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

pub fn with_alpha(color: Rgb, alpha: f64) -> String {
    format!(
        "{}{:02x}",
        hex(color),
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

pub fn parse_hex(value: &str) -> Option<Rgb> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    Some([
        ((number >> 16) & 255) as u8,
        ((number >> 8) & 255) as u8,
        (number & 255) as u8,
    ])
}

pub fn luminance([r, g, b]: Rgb) -> f64 {
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (high, low) = if luminance(a) >= luminance(b) {
        (luminance(a), luminance(b))
    } else {
        (luminance(b), luminance(a))
    };
    (high + 0.05) / (low + 0.05)
}

pub fn is_dark(color: Rgb) -> bool {
    luminance(color) < 0.25
}

pub fn mix(from: Rgb, to: Rgb, amount: f64) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    std::array::from_fn(|index| {
        (f64::from(from[index]) + (f64::from(to[index]) - f64::from(from[index])) * amount).round()
            as u8
    })
}

pub fn shade(base: Rgb, amount: f64) -> Rgb {
    let size = amount.abs();
    let up = amount >= 0.0;
    let maximum = f64::from(*base.iter().max().unwrap());
    let minimum = f64::from(*base.iter().min().unwrap());
    let room = if up { 255.0 - maximum } else { minimum };
    let direction = if room >= size {
        if up { 1.0 } else { -1.0 }
    } else if up {
        -1.0
    } else {
        1.0
    };
    std::array::from_fn(|index| {
        (f64::from(base[index]) + direction * size)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

pub fn legible(color: Rgb, on: Rgb, target: f64) -> Rgb {
    if contrast(color, on) >= target {
        return color;
    }
    let lighten = luminance(on) < 0.5;
    let original = to_oklch(color);
    let mut best = color;
    for step in 1..=40 {
        let l = (original.l + if lighten { 1.0 } else { -1.0 } * f64::from(step) * 0.02)
            .clamp(0.0, 1.0);
        best = from_oklch(Oklch {
            l,
            c: original.c,
            h: original.h,
        });
        if contrast(best, on) >= target {
            return best;
        }
    }
    best
}

fn to_linear(channel: u8) -> f64 {
    let channel = f64::from(channel) / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(channel: f64) -> u8 {
    let channel = if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    };
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_alpha_and_parsing_are_stable() {
        assert_eq!(hex([0, 128, 255]), "#0080ff");
        assert_eq!(with_alpha([0, 128, 255], 0.5), "#0080ff80");
        assert_eq!(parse_hex(" #0080Ff "), Some([0, 128, 255]));
        assert_eq!(parse_hex("#abc"), None);
    }

    #[test]
    fn oklch_round_trips_representative_colours() {
        for color in [[0, 0, 0], [255, 255, 255], [229, 72, 77], [93, 156, 255]] {
            let round_trip = from_oklch(to_oklch(color));
            for channel in 0..3 {
                assert!((i16::from(round_trip[channel]) - i16::from(color[channel])).abs() <= 1);
            }
        }
    }

    #[test]
    fn shade_steps_away_even_at_black_and_white() {
        assert_eq!(shade([0, 0, 0], 6.0), [6, 6, 6]);
        assert_eq!(shade([255, 255, 255], 6.0), [249, 249, 249]);
        assert_eq!(shade([0, 0, 0], -10.0), [10, 10, 10]);
    }

    #[test]
    fn legible_reaches_requested_contrast() {
        let adjusted = legible([40, 40, 40], [20, 20, 20], 4.5);
        assert!(contrast(adjusted, [20, 20, 20]) >= 4.5);
    }
}
