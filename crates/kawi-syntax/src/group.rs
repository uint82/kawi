#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxGroup {
    Comment,
    Keyword,
    String,
    Number,
    Constant,
    Function,
    Type,
    Variable,
    Property,
    Operator,
    Punctuation,
    Attribute,
    Tag,
    Invalid,
}

impl SyntaxGroup {
    pub const ALL: [Self; 14] = [
        Self::Comment,
        Self::Keyword,
        Self::String,
        Self::Number,
        Self::Constant,
        Self::Function,
        Self::Type,
        Self::Variable,
        Self::Property,
        Self::Operator,
        Self::Punctuation,
        Self::Attribute,
        Self::Tag,
        Self::Invalid,
    ];

    pub fn from_capture(name: &str) -> Option<Self> {
        match name.split('.').next()? {
            "comment" => Some(Self::Comment),
            "keyword" => Some(Self::Keyword),
            "string" | "escape" | "character" => Some(Self::String),
            "number" => Some(Self::Number),
            "boolean" | "constant" | "special" => Some(Self::Constant),
            "function" => Some(Self::Function),
            "constructor" | "type" | "module" | "namespace" => Some(Self::Type),
            "variable" | "label" => Some(Self::Variable),
            "property" => Some(Self::Property),
            "operator" => Some(Self::Operator),
            "punctuation" => Some(Self::Punctuation),
            "attribute" | "decorator" => Some(Self::Attribute),
            "tag" => Some(Self::Tag),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_captures() {
        assert_eq!(
            SyntaxGroup::from_capture("comment"),
            Some(SyntaxGroup::Comment)
        );
        assert_eq!(
            SyntaxGroup::from_capture("comment.documentation"),
            Some(SyntaxGroup::Comment)
        );
        assert_eq!(
            SyntaxGroup::from_capture("keyword"),
            Some(SyntaxGroup::Keyword)
        );
        assert_eq!(
            SyntaxGroup::from_capture("string.special.key"),
            Some(SyntaxGroup::String)
        );
        assert_eq!(
            SyntaxGroup::from_capture("escape"),
            Some(SyntaxGroup::String)
        );
        assert_eq!(
            SyntaxGroup::from_capture("number"),
            Some(SyntaxGroup::Number)
        );
        assert_eq!(
            SyntaxGroup::from_capture("constant.builtin"),
            Some(SyntaxGroup::Constant)
        );
        assert_eq!(
            SyntaxGroup::from_capture("boolean"),
            Some(SyntaxGroup::Constant)
        );
        assert_eq!(
            SyntaxGroup::from_capture("function.method"),
            Some(SyntaxGroup::Function)
        );
        assert_eq!(
            SyntaxGroup::from_capture("type.builtin"),
            Some(SyntaxGroup::Type)
        );
        assert_eq!(
            SyntaxGroup::from_capture("variable.parameter"),
            Some(SyntaxGroup::Variable)
        );
        assert_eq!(
            SyntaxGroup::from_capture("punctuation.bracket"),
            Some(SyntaxGroup::Punctuation)
        );
        assert_eq!(SyntaxGroup::from_capture("tag"), Some(SyntaxGroup::Tag));
        assert_eq!(
            SyntaxGroup::from_capture("attribute"),
            Some(SyntaxGroup::Attribute)
        );
    }

    #[test]
    fn ignores_unknown_captures() {
        assert_eq!(SyntaxGroup::from_capture("embedded"), None);
        assert_eq!(SyntaxGroup::from_capture(""), None);
        assert_eq!(SyntaxGroup::from_capture("spell"), None);
    }

    #[test]
    fn declares_every_group_once() {
        let mut groups = SyntaxGroup::ALL.to_vec();
        groups.sort_by_key(|group| *group as usize);
        groups.dedup();
        assert_eq!(groups.len(), SyntaxGroup::ALL.len());
    }
}
