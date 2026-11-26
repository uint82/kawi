mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages.rs"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelpPage {
    pub slug: &'static str,
    pub title: &'static str,
    pub aliases: &'static [&'static str],
    pub related: &'static [&'static str],
    pub codes: &'static [&'static str],
    body: &'static str,
    body_start: usize,
}

impl HelpPage {
    pub fn body(self) -> &'static str {
        &self.body[self.body_start..]
    }
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
            codes: page.codes,
            body: page.body,
            body_start: page.body_start,
        })
    }

    pub fn page(self, slug: &str) -> Option<HelpPage> {
        self.pages().find(|page| page.slug == slug)
    }

    pub fn resolve(self, topic: &str) -> Option<HelpPage> {
        let query = normalize_topic(topic);
        let slug = query.to_ascii_lowercase().replace([' ', '_'], "-");
        self.pages()
            .find(|page| {
                page.slug == slug
                    || page
                        .aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(query))
                    || page.title.eq_ignore_ascii_case(query)
            })
            .or_else(|| self.code_page(query))
    }

    pub fn code(self, topic: &str) -> Option<&'static str> {
        let query = normalize_topic(topic);
        self.codes().find(|code| code.eq_ignore_ascii_case(query))
    }

    pub fn codes(self) -> impl Iterator<Item = &'static str> {
        self.pages().flat_map(|page| page.codes.iter().copied())
    }

    fn code_page(self, query: &str) -> Option<HelpPage> {
        self.pages().find(|page| {
            page.codes
                .iter()
                .any(|code| code.eq_ignore_ascii_case(query))
        })
    }
}

fn normalize_topic(topic: &str) -> &str {
    topic.trim().trim_start_matches("help:")
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
    fn error_codes_resolve_to_the_code_page() {
        let index = HelpIndex;
        assert_eq!(index.resolve("E492").expect("E492").slug, "errors");
        assert_eq!(index.resolve("k001").expect("K001").slug, "errors");
        assert_eq!(index.code("e492"), Some("E492"));
        assert_eq!(index.code("K001"), Some("K001"));
        assert_eq!(index.resolve("E9999"), None);
    }

    #[test]
    fn error_codes_are_unique_and_uppercase() {
        let index = HelpIndex;
        let mut seen = std::collections::BTreeSet::new();
        for code in index.codes() {
            assert_eq!(code, code.to_ascii_uppercase());
            assert!(seen.insert(code), "duplicate code {code}");
        }
    }
}
