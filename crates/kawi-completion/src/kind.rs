#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl CompletionKind {
    pub const ALL: [Self; 25] = [
        Self::Text,
        Self::Method,
        Self::Function,
        Self::Constructor,
        Self::Field,
        Self::Variable,
        Self::Class,
        Self::Interface,
        Self::Module,
        Self::Property,
        Self::Unit,
        Self::Value,
        Self::Enum,
        Self::Keyword,
        Self::Snippet,
        Self::Color,
        Self::File,
        Self::Reference,
        Self::Folder,
        Self::EnumMember,
        Self::Constant,
        Self::Struct,
        Self::Event,
        Self::Operator,
        Self::TypeParameter,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Method => "method",
            Self::Function => "function",
            Self::Constructor => "constructor",
            Self::Field => "field",
            Self::Variable => "variable",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Module => "module",
            Self::Property => "property",
            Self::Unit => "unit",
            Self::Value => "value",
            Self::Enum => "enum",
            Self::Keyword => "keyword",
            Self::Snippet => "snippet",
            Self::Color => "color",
            Self::File => "file",
            Self::Reference => "reference",
            Self::Folder => "folder",
            Self::EnumMember => "enum-member",
            Self::Constant => "constant",
            Self::Struct => "struct",
            Self::Event => "event",
            Self::Operator => "operator",
            Self::TypeParameter => "type-parameter",
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::EnumMember => "enum_member",
            Self::TypeParameter => "type_parameter",
            _ => self.name(),
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_kind_has_a_unique_name_and_key() {
        let names: BTreeSet<_> = CompletionKind::ALL.iter().map(|kind| kind.name()).collect();
        let keys: BTreeSet<_> = CompletionKind::ALL.iter().map(|kind| kind.key()).collect();
        assert_eq!(names.len(), CompletionKind::ALL.len());
        assert_eq!(keys.len(), CompletionKind::ALL.len());
    }

    #[test]
    fn keys_round_trip() {
        for kind in CompletionKind::ALL {
            assert_eq!(CompletionKind::from_key(kind.key()), Some(kind));
        }
    }

    #[test]
    fn unknown_keys_are_ignored() {
        assert_eq!(CompletionKind::from_key("nope"), None);
        assert_eq!(CompletionKind::from_key("TypeParameter"), None);
    }

    #[test]
    fn indexes_are_unique() {
        let indexes: BTreeSet<_> = CompletionKind::ALL
            .iter()
            .map(|kind| kind.index())
            .collect();
        assert_eq!(indexes.len(), CompletionKind::ALL.len());
    }
}
