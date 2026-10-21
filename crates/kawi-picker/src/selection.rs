pub fn clamped_selection(
    current: Option<usize>,
    length: usize,
    forward: bool,
    amount: usize,
) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let current = current.unwrap_or(0);
    Some(if forward {
        current.saturating_add(amount).min(length - 1)
    } else {
        current.saturating_sub(amount)
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
    fn clamps_to_both_ends() {
        assert_eq!(clamped_selection(Some(0), 3, false, 1), Some(0));
        assert_eq!(clamped_selection(Some(2), 3, true, 1), Some(2));
        assert_eq!(clamped_selection(Some(1), 3, true, 5), Some(2));
        assert_eq!(clamped_selection(None, 0, true, 1), None);
    }

    #[test]
    fn clamps_an_explicit_index() {
        assert_eq!(selection_at(9, 3), Some(2));
        assert_eq!(selection_at(0, 0), None);
    }
}
