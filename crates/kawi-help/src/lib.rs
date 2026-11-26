mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages.rs"));
}

mod code_registry {
    include!(concat!(env!("OUT_DIR"), "/codes.rs"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelpPage {
    pub slug: &'static str,
    pub title: &'static str,
    pub aliases: &'static [&'static str],
    pub related: &'static [&'static str],
    body: &'static str,
    body_start: usize,
}

impl HelpPage {
    pub fn body(self) -> &'static str {
        &self.body[self.body_start..]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelpTarget {
    pub page: HelpPage,
    pub code: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HelpIndex;

impl HelpIndex {
    pub fn pages(self) -> impl Iterator<Item = HelpPage> {
        pages::PAGE_FILES.iter().map(|page| HelpPage {
            slug: page.slug,
            title: page.title,
            aliases: page.aliases,
            related: page.related,
            body: page.body,
            body_start: page.body_start,
        })
    }

    pub fn page(self, slug: &str) -> Option<HelpPage> {
        self.pages().find(|page| page.slug == slug)
    }

    pub fn lookup(self, topic: &str) -> Option<HelpTarget> {
        let query = normalize_topic(topic);
        if let Some(page) = self.page_topic(query) {
            return Some(HelpTarget { page, code: None });
        }
        let entry = code_entry(query)?;
        let page = self.page(entry.page)?;
        Some(HelpTarget {
            page,
            code: Some(entry.code),
        })
    }

    pub fn resolve(self, topic: &str) -> Option<HelpPage> {
        self.lookup(topic).map(|target| target.page)
    }

    pub fn codes(self) -> impl Iterator<Item = &'static str> {
        code_registry::CODE_FILES.iter().map(|entry| entry.code)
    }

    fn page_topic(self, query: &str) -> Option<HelpPage> {
        let slug = query.to_ascii_lowercase().replace([' ', '_'], "-");
        self.pages().find(|page| {
            page.slug == slug
                || page
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(query))
                || page.title.eq_ignore_ascii_case(query)
        })
    }
}

fn normalize_topic(topic: &str) -> &str {
    topic.trim().trim_start_matches("help:").trim()
}

fn code_entry(query: &str) -> Option<&'static code_registry::CodeFile> {
    let query = query.to_ascii_uppercase();
    code_registry::CODE_FILES
        .binary_search_by(|entry| entry.code.cmp(query.as_str()))
        .ok()
        .map(|index| &code_registry::CODE_FILES[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_resolves_home_and_alias() {
        let index = HelpIndex;
        assert_eq!(index.page("index").expect("index").slug, "index");
        assert_eq!(index.resolve("home").expect("home").slug, "index");
        assert_eq!(
            index.resolve("start").expect("start").slug,
            "getting-started"
        );
        assert_eq!(
            index.resolve("Getting started").expect("title").slug,
            "getting-started"
        );
    }

    #[test]
    fn body_excludes_frontmatter() {
        let page = HelpIndex.page("index").expect("index");
        assert!(page.body().starts_with("# Kawi Help"));
    }

    #[test]
    fn help_links_resolve() {
        let index = HelpIndex;
        for page in index.pages() {
            let mut rest = page.body();
            while let Some(start) = rest.find("](help:") {
                let target = &rest[start + 7..];
                let end = target.find(')').expect("closed help link");
                let topic = &target[..end];
                assert!(
                    index.resolve(topic).is_some(),
                    "{} links to missing {topic}",
                    page.slug
                );
                rest = &target[end + 1..];
            }
        }
    }

    #[test]
    fn aliases_are_unique() {
        let index = HelpIndex;
        let mut seen = std::collections::BTreeSet::new();
        for page in index.pages() {
            assert!(seen.insert(page.slug), "duplicate slug {}", page.slug);
            for alias in page.aliases {
                assert!(seen.insert(*alias), "duplicate alias {alias}");
            }
        }
    }

    #[test]
    fn lookup_resolves_error_codes_to_the_reference_page() {
        let index = HelpIndex;
        let target = index.lookup("E492").expect("E492");
        assert_eq!(target.page.slug, "errors");
        assert_eq!(target.code, Some("E492"));
        let target = index.lookup("k001").expect("K001");
        assert_eq!(target.page.slug, "errors");
        assert_eq!(target.code, Some("K001"));
        assert!(index.lookup("E9999").is_none());
    }

    #[test]
    fn lookup_normalizes_whitespace_and_the_help_prefix() {
        let target = HelpIndex.lookup("  help:e492  ").expect("E492");
        assert_eq!(target.code, Some("E492"));
    }

    #[test]
    fn lookup_returns_page_topics_without_a_code() {
        let target = HelpIndex.lookup("errors").expect("errors");
        assert_eq!(target.page.slug, "errors");
        assert_eq!(target.code, None);
        assert_eq!(HelpIndex.lookup("help:errors").expect("errors").code, None);
    }

    #[test]
    fn lookup_requires_the_whole_code() {
        let index = HelpIndex;
        assert!(index.lookup("").is_none());
        assert!(index.lookup("help:").is_none());
        assert!(index.lookup("E49").is_none());
        assert!(index.lookup("E4920").is_none());
        assert!(index.lookup("E492: Not an editor command").is_none());
    }

    #[test]
    fn codes_are_sorted_unique_and_uppercase() {
        let codes: Vec<_> = HelpIndex.codes().collect();
        assert!(!codes.is_empty());
        assert!(codes.windows(2).all(|pair| pair[0] < pair[1]));
        for code in codes {
            assert_eq!(code, code.to_ascii_uppercase());
        }
    }
}
