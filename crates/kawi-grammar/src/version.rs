use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<Identifier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Identifier {
    Numeric(u64),
    Alpha(String),
}

impl Version {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let (without_build, build) = match value.split_once('+') {
            Some((left, right)) => (left, Some(right)),
            None => (value, None),
        };
        if build.is_some_and(|build| !is_identifier_list(build)) {
            return None;
        }
        let (core, pre) = match without_build.split_once('-') {
            Some((left, right)) => (left, Some(right)),
            None => (without_build, None),
        };
        let mut parts = core.split('.');
        let major = parse_numeric(parts.next()?)?;
        let minor = parse_numeric(parts.next()?)?;
        let patch = parse_numeric(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            Some(pre) => parse_pre(pre)?,
            None => Vec::new(),
        };
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    pub(crate) fn is_valid(value: &str) -> bool {
        Self::parse(value).is_some()
    }
}

pub(crate) fn compare(left: &str, right: &str) -> Option<Ordering> {
    Some(Version::parse(left)?.cmp(&Version::parse(right)?))
}

fn parse_numeric(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

fn parse_pre(value: &str) -> Option<Vec<Identifier>> {
    if value.is_empty() {
        return None;
    }
    let mut identifiers = Vec::new();
    for part in value.split('.') {
        if part.is_empty() {
            return None;
        }
        if part.bytes().all(|byte| byte.is_ascii_digit()) {
            identifiers.push(Identifier::Numeric(parse_numeric(part)?));
        } else if is_identifier(part) {
            identifiers.push(Identifier::Alpha(part.to_string()));
        } else {
            return None;
        }
    }
    Some(identifiers)
}

fn is_identifier_list(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_identifier)
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| compare_pre(&self.pre, &other.pre))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_pre(left: &[Identifier], right: &[Identifier]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            for (left, right) in left.iter().zip(right) {
                let order = compare_identifier(left, right);
                if order != Ordering::Equal {
                    return order;
                }
            }
            left.len().cmp(&right.len())
        }
    }
}

fn compare_identifier(left: &Identifier, right: &Identifier) -> Ordering {
    match (left, right) {
        (Identifier::Numeric(left), Identifier::Numeric(right)) => left.cmp(right),
        (Identifier::Numeric(_), Identifier::Alpha(_)) => Ordering::Less,
        (Identifier::Alpha(_), Identifier::Numeric(_)) => Ordering::Greater,
        (Identifier::Alpha(left), Identifier::Alpha(right)) => left.cmp(right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_version() {
        assert!(Version::is_valid("0.0.1"));
    }

    #[test]
    fn accepts_prerelease_and_build_metadata() {
        assert!(Version::is_valid("1.2.3-alpha.1+build.5"));
    }

    #[test]
    fn rejects_leading_zeroes_in_the_core() {
        assert!(!Version::is_valid("01.2.3"));
    }

    #[test]
    fn rejects_leading_zeroes_in_numeric_prereleases() {
        assert!(!Version::is_valid("1.2.3-01"));
    }

    #[test]
    fn rejects_missing_components() {
        assert!(!Version::is_valid("1.2"));
        assert!(!Version::is_valid("1.2.3.4"));
    }

    #[test]
    fn rejects_path_characters() {
        assert!(!Version::is_valid("../1.2.3"));
        assert!(!Version::is_valid("1.2.3/4"));
    }

    #[test]
    fn orders_core_versions() {
        assert_eq!(compare("1.2.3", "1.2.4"), Some(Ordering::Less));
        assert_eq!(compare("2.0.0", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(compare("1.2.3", "1.2.3"), Some(Ordering::Equal));
    }

    #[test]
    fn prereleases_sort_before_releases() {
        assert_eq!(compare("1.2.3-alpha", "1.2.3"), Some(Ordering::Less));
        assert_eq!(compare("1.2.3", "1.2.3-alpha"), Some(Ordering::Greater));
    }

    #[test]
    fn numeric_prerelease_identifiers_sort_before_alpha() {
        assert_eq!(compare("1.2.3-1", "1.2.3-alpha"), Some(Ordering::Less));
    }

    #[test]
    fn longer_prerelease_lists_sort_after_their_prefix() {
        assert_eq!(
            compare("1.2.3-alpha", "1.2.3-alpha.1"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn build_metadata_is_ignored() {
        assert_eq!(compare("1.2.3+one", "1.2.3+two"), Some(Ordering::Equal));
    }
}
