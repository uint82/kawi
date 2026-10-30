use crate::types::Position;

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        text.bytes()
            .enumerate()
            .filter(|(_, byte)| *byte == b'\n')
            .map(|(index, _)| index + 1),
    );
    starts
}

pub fn position_at(text: &str, byte_offset: usize) -> Option<Position> {
    if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
        return None;
    }
    let starts = line_starts(text);
    let line = starts.partition_point(|start| *start <= byte_offset) - 1;
    Some(Position {
        line: u32::try_from(line).ok()?,
        byte_column: u32::try_from(byte_offset - starts[line]).ok()?,
    })
}

pub fn byte_offset(text: &str, position: Position) -> Option<usize> {
    let starts = line_starts(text);
    let line = usize::try_from(position.line).ok()?;
    let start = *starts.get(line)?;
    let end = starts
        .get(line + 1)
        .map_or(text.len(), |next| next.saturating_sub(1));
    let column = usize::try_from(position.byte_column).ok()?;
    let offset = start.checked_add(column)?;
    if offset > end || !text.is_char_boundary(offset) {
        return None;
    }
    Some(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_use_utf8_byte_columns() {
        let start = position_at("é\nbeta", 2).unwrap();
        assert_eq!((start.line, start.byte_column), (0, 2));
        let next = position_at("é\nbeta", 3).unwrap();
        assert_eq!((next.line, next.byte_column), (1, 0));
    }

    #[test]
    fn byte_offsets_round_trip_positions() {
        let text = "é\nbeta\n";
        for offset in [0, 2, 3, 7, 8] {
            let position = position_at(text, offset).unwrap();
            assert_eq!(byte_offset(text, position), Some(offset));
        }
    }

    #[test]
    fn rejects_offsets_inside_a_codepoint() {
        assert!(position_at("é", 1).is_none());
    }
}
