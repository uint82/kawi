use std::cmp::Ordering;

use super::entry::Entry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum SortMode {
    #[default]
    DirectoriesFirst,
    Name,
}

impl SortMode {
    pub(in crate::app) fn parse(value: &str) -> Option<Self> {
        match value {
            "directories" => Some(Self::DirectoriesFirst),
            "name" => Some(Self::Name),
            _ => None,
        }
    }

    pub(in crate::app) fn name(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "directories",
            Self::Name => "name",
        }
    }
}

pub(super) fn compare_entries(left: &Entry, right: &Entry, sort: SortMode) -> Ordering {
    let directories = if sort == SortMode::DirectoriesFirst {
        right.is_directory().cmp(&left.is_directory())
    } else {
        Ordering::Equal
    };
    directories
        .then_with(|| natural_cmp(&left.name.to_string_lossy(), &right.name.to_string_lossy()))
        .then_with(|| left.name.cmp(&right.name))
}

pub(super) fn natural_cmp(left_text: &str, right_text: &str) -> Ordering {
    let left: Vec<char> = left_text.chars().collect();
    let right: Vec<char> = right_text.chars().collect();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        let left_char = left[left_index];
        let right_char = right[right_index];
        if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
            let left_end = digit_end(&left, left_index);
            let right_end = digit_end(&right, right_index);
            let left_digits = trim_zeroes(&left[left_index..left_end]);
            let right_digits = trim_zeroes(&right[right_index..right_end]);
            let ordering = left_digits
                .len()
                .cmp(&right_digits.len())
                .then_with(|| left_digits.cmp(right_digits))
                .then_with(|| (left_end - left_index).cmp(&(right_end - right_index)));
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }
        let ordering = lower(left_char).cmp(&lower(right_char));
        if ordering != Ordering::Equal {
            return ordering;
        }
        left_index += 1;
        right_index += 1;
    }
    (left.len() - left_index)
        .cmp(&(right.len() - right_index))
        .then_with(|| left_text.cmp(right_text))
}

fn digit_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    end
}

fn trim_zeroes(digits: &[char]) -> &[char] {
    let start = digits
        .iter()
        .position(|digit| *digit != '0')
        .unwrap_or(digits.len());
    &digits[start..]
}

fn lower(character: char) -> char {
    character.to_lowercase().next().unwrap_or(character)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    #[test]
    fn natural_order_handles_numbers() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("File10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("file02", "file2"), Ordering::Greater);
    }
}
