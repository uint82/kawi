mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages.rs"));
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

    pub fn resolve(self, topic: &str) -> Option<HelpPage> {
        let query = topic.trim().trim_start_matches("help:");
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
}
