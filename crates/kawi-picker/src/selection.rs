pub fn wrapped_selection(
    current: Option<usize>,
    length: usize,
    forward: bool,
    amount: usize,
) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let current = current.unwrap_or(0).min(length - 1);
    let offset = amount % length;
    Some(if forward {
        current.saturating_add(offset) % length
    } else {
        current.saturating_add(length).saturating_sub(offset) % length
    })
}

pub fn selection_at(index: usize, length: usize) -> Option<usize> {
    if length == 0 {
        None
    } else {
        Some(index.min(length - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_both_ends() {
        assert_eq!(wrapped_selection(Some(2), 3, true, 1), Some(0));
        assert_eq!(wrapped_selection(Some(0), 3, false, 1), Some(2));
        assert_eq!(wrapped_selection(Some(1), 3, true, 5), Some(0));
        assert_eq!(wrapped_selection(Some(1), 3, true, 3), Some(1));
        assert_eq!(wrapped_selection(None, 0, true, 1), None);
    }

    #[test]
    fn clamps_an_explicit_index() {
        assert_eq!(selection_at(9, 3), Some(2));
        assert_eq!(selection_at(0, 0), None);
    }
}
