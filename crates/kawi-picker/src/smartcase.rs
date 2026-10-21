use regex_syntax::ast::{self, parse::Parser, Ast, ClassSet, ClassSetItem};

pub(crate) fn case_insensitive(pattern: &str) -> bool {
    let Ok(ast) = Parser::new().parse(pattern) else {
        return false;
    };
    let analysis = Analysis::new(&ast);
    analysis.any_literal && !analysis.any_uppercase
}

struct Analysis {
    any_literal: bool,
    any_uppercase: bool,
}

impl Analysis {
    fn new(ast: &Ast) -> Self {
        let mut analysis = Self {
            any_literal: false,
            any_uppercase: false,
        };
        analysis.visit(ast);
        analysis
    }

    fn visit(&mut self, ast: &Ast) {
        if self.done() {
            return;
        }
        match ast {
            Ast::Literal(literal) => self.mark_literal(literal),
            Ast::ClassBracketed(class) => self.visit_class_set(&class.kind),
            Ast::Repetition(repetition) => self.visit(&repetition.ast),
            Ast::Group(group) => self.visit(&group.ast),
            Ast::Alternation(alternation) => {
                for ast in &alternation.asts {
                    self.visit(ast);
                }
            }
            Ast::Concat(concat) => {
                for ast in &concat.asts {
                    self.visit(ast);
                }
            }
            _ => {}
        }
    }

    fn visit_class_set(&mut self, set: &ClassSet) {
        if self.done() {
            return;
        }
        match set {
            ClassSet::Item(item) => self.visit_class_item(item),
            ClassSet::BinaryOp(operation) => {
                self.visit_class_set(&operation.lhs);
                self.visit_class_set(&operation.rhs);
            }
        }
    }

    fn visit_class_item(&mut self, item: &ClassSetItem) {
        if self.done() {
            return;
        }
        match item {
            ClassSetItem::Literal(literal) => self.mark_literal(literal),
            ClassSetItem::Range(range) => {
                self.mark_literal(&range.start);
                self.mark_literal(&range.end);
            }
            ClassSetItem::Bracketed(class) => self.visit_class_set(&class.kind),
            ClassSetItem::Union(union) => {
                for item in &union.items {
                    self.visit_class_item(item);
                }
            }
            _ => {}
        }
    }

    fn mark_literal(&mut self, literal: &ast::Literal) {
        self.any_literal = true;
        self.any_uppercase = self.any_uppercase || literal.c.is_uppercase();
    }

    fn done(&self) -> bool {
        self.any_literal && self.any_uppercase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_ripgrep_smartcase_analysis() {
        for pattern in ["foo", "foo\\w", "foo\\S", "foo\\p{Ll}", "foo[a-z]"] {
            assert!(case_insensitive(pattern), "{pattern}");
        }
        for pattern in ["Foo", "foO", "foo[A-Z]", "foo\\\\S", "aBc\\w"] {
            assert!(!case_insensitive(pattern), "{pattern}");
        }
        for pattern in ["\\p{Lu}", "\\S", ""] {
            assert!(!case_insensitive(pattern), "{pattern}");
        }
    }

    #[test]
    fn invalid_patterns_are_not_insensitive() {
        assert!(!case_insensitive("("));
    }
}
