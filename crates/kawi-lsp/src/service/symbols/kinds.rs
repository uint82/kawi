#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    Unknown,
}

impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Package => "package",
            Self::Class => "class",
            Self::Method => "method",
            Self::Property => "property",
            Self::Field => "field",
            Self::Constructor => "constructor",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Function => "function",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
            Self::Key => "key",
            Self::Null => "null",
            Self::EnumMember => "enum member",
            Self::Struct => "struct",
            Self::Event => "event",
            Self::Operator => "operator",
            Self::TypeParameter => "type parameter",
            Self::Unknown => "symbol",
        }
    }

    pub(super) fn from_lsp(kind: lsp_types::SymbolKind) -> Self {
        match kind {
            lsp_types::SymbolKind::FILE => Self::File,
            lsp_types::SymbolKind::MODULE => Self::Module,
            lsp_types::SymbolKind::NAMESPACE => Self::Namespace,
            lsp_types::SymbolKind::PACKAGE => Self::Package,
            lsp_types::SymbolKind::CLASS => Self::Class,
            lsp_types::SymbolKind::METHOD => Self::Method,
            lsp_types::SymbolKind::PROPERTY => Self::Property,
            lsp_types::SymbolKind::FIELD => Self::Field,
            lsp_types::SymbolKind::CONSTRUCTOR => Self::Constructor,
            lsp_types::SymbolKind::ENUM => Self::Enum,
            lsp_types::SymbolKind::INTERFACE => Self::Interface,
            lsp_types::SymbolKind::FUNCTION => Self::Function,
            lsp_types::SymbolKind::VARIABLE => Self::Variable,
            lsp_types::SymbolKind::CONSTANT => Self::Constant,
            lsp_types::SymbolKind::STRING => Self::String,
            lsp_types::SymbolKind::NUMBER => Self::Number,
            lsp_types::SymbolKind::BOOLEAN => Self::Boolean,
            lsp_types::SymbolKind::ARRAY => Self::Array,
            lsp_types::SymbolKind::OBJECT => Self::Object,
            lsp_types::SymbolKind::KEY => Self::Key,
            lsp_types::SymbolKind::NULL => Self::Null,
            lsp_types::SymbolKind::ENUM_MEMBER => Self::EnumMember,
            lsp_types::SymbolKind::STRUCT => Self::Struct,
            lsp_types::SymbolKind::EVENT => Self::Event,
            lsp_types::SymbolKind::OPERATOR => Self::Operator,
            lsp_types::SymbolKind::TYPE_PARAMETER => Self::TypeParameter,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kinds_get_a_default_label() {
        let kind =
            serde_json::from_value::<lsp_types::SymbolKind>(serde_json::json!(999)).expect("kind");
        assert_eq!(SymbolKind::from_lsp(kind).label(), "symbol");
    }

    #[test]
    fn known_kinds_get_text_labels() {
        assert_eq!(
            SymbolKind::from_lsp(lsp_types::SymbolKind::FUNCTION).label(),
            "function"
        );
    }
}
