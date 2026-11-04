use crossterm::style::Color;

use super::ThemeError;

pub(super) fn parse_color(field: &str, value: &str) -> Result<Color, ThemeError> {
    let value = value.trim();
    let normalized = value.to_ascii_lowercase();
    if let Some(hex) = normalized.strip_prefix('#') {
        return parse_hex(field, value, hex);
    }
    match normalized.as_str() {
        "black" => Ok(Color::Black),
        "dark-grey" | "dark-gray" => Ok(Color::DarkGrey),
        "red" => Ok(Color::Red),
        "dark-red" => Ok(Color::DarkRed),
        "green" => Ok(Color::Green),
        "dark-green" => Ok(Color::DarkGreen),
        "yellow" => Ok(Color::Yellow),
        "dark-yellow" => Ok(Color::DarkYellow),
        "blue" => Ok(Color::Blue),
        "dark-blue" => Ok(Color::DarkBlue),
        "magenta" => Ok(Color::Magenta),
        "dark-magenta" => Ok(Color::DarkMagenta),
        "cyan" => Ok(Color::Cyan),
        "dark-cyan" => Ok(Color::DarkCyan),
        "white" => Ok(Color::White),
        "grey" | "gray" => Ok(Color::Grey),
        "reset" | "default" => Ok(Color::Reset),
        _ => Err(ThemeError::invalid(format!(
            "{field} must be a hex color or a terminal color name, got {value:?}"
        ))),
    }
}

fn parse_hex(field: &str, value: &str, hex: &str) -> Result<Color, ThemeError> {
    if !hex.is_ascii() {
        return Err(invalid_hex(field, value));
    }
    let expanded = match hex.len() {
        3 => hex
            .chars()
            .flat_map(|digit| [digit, digit])
            .collect::<String>(),
        6 => hex.to_string(),
        _ => return Err(invalid_hex(field, value)),
    };
    let r = u8::from_str_radix(&expanded[0..2], 16).map_err(|_| invalid_hex(field, value))?;
    let g = u8::from_str_radix(&expanded[2..4], 16).map_err(|_| invalid_hex(field, value))?;
    let b = u8::from_str_radix(&expanded[4..6], 16).map_err(|_| invalid_hex(field, value))?;
    Ok(Color::Rgb { r, g, b })
}

fn invalid_hex(field: &str, value: &str) -> ThemeError {
    ThemeError::invalid(format!("{field} must be #rgb or #rrggbb, got {value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(
            parse_color("base.foreground", "#e0e2ea").unwrap(),
            Color::Rgb {
                r: 224,
                g: 226,
                b: 234
            }
        );
        assert_eq!(
            parse_color("base.background", "#abc").unwrap(),
            Color::Rgb {
                r: 170,
                g: 187,
                b: 204
            }
        );
    }

    #[test]
    fn parses_named_colors() {
        assert_eq!(parse_color("base.foreground", "red").unwrap(), Color::Red);
        assert_eq!(
            parse_color("base.background", "dark-grey").unwrap(),
            Color::DarkGrey
        );
        assert_eq!(
            parse_color("base.background", "reset").unwrap(),
            Color::Reset
        );
    }

    #[test]
    fn rejects_invalid_colors() {
        assert!(parse_color("base.foreground", "blueish").is_err());
        assert!(parse_color("base.foreground", "#12").is_err());
        assert!(parse_color("base.foreground", "#gggggg").is_err());
        assert!(parse_color("base.foreground", "#12345678").is_err());
        assert!(parse_color("base.foreground", "#€").is_err());
        assert!(parse_color("base.foreground", "#€€€").is_err());
    }
}
