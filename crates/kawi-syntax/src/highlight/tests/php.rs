use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_php_namespaces_and_imports() {
    let text = "\
<?php
namespace App\\Demo;
use Other\\Thing;
use Other\\Thing as Alias;
use function Other\\helper;
use const Other\\VALUE;
use TraitOne;
";
    let spans = highlight(LanguageId::Php, text, 0, 7);
    assert_eq!(name_of(&spans, text, 1, "namespace"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 1, "App"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "Demo"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 2, "Other"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "Thing"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "Alias"), Some("type.definition"));
    assert_eq!(
        name_of(&spans, text, 4, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 4, "helper"), Some("function"));
    assert_eq!(name_of(&spans, text, 5, "const"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 5, "VALUE"), Some("constant"));
    assert_eq!(name_of(&spans, text, 6, "TraitOne"), Some("type"));
}

#[test]
fn highlights_php_declarations_and_types() {
    let text = "\
<?php
#[Marker]
final class Sample extends Base implements Contract {
    use TraitOne, TraitTwo;
    public const VERSION = '1.0';
}
interface Contract {}
trait Reusable {}
enum Status: string {
    case Active = 'active';
}
";
    let spans = highlight(LanguageId::Php, text, 0, 11);
    assert_eq!(name_of(&spans, text, 1, "#["), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 1, "Marker"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 2, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 2, "Sample"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 2, "extends"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "Base"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "implements"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "Contract"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 3, "TraitOne"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "TraitTwo"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "public"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 4, "const"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 4, "VERSION"), Some("constant"));
    assert_eq!(name_of(&spans, text, 6, "interface"), Some("keyword.type"));
    assert_eq!(
        name_of(&spans, text, 6, "Contract"),
        Some("type.definition")
    );
    assert_eq!(name_of(&spans, text, 7, "trait"), Some("keyword.type"));
    assert_eq!(
        name_of(&spans, text, 7, "Reusable"),
        Some("type.definition")
    );
    assert_eq!(name_of(&spans, text, 8, "enum"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 8, "Status"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 8, "string"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 9, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 9, "Active"), Some("constant"));
}

#[test]
fn highlights_php_functions_methods_and_parameters() {
    let text = "\
<?php
function helper(int ...$values): void {}
class Service {
    public function __construct(private readonly int $count = 0) {}
    public function run(?Service $other): string {
        $this->method($other);
        return self::class;
    }
}
";
    let spans = highlight(LanguageId::Php, text, 0, 9);
    assert_eq!(name_of(&spans, text, 1, "helper"), Some("function"));
    assert_eq!(name_of(&spans, text, 1, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "..."), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 1, "values"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "void"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "Service"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 3, "__construct"), Some("constructor"));
    assert_eq!(
        name_of(&spans, text, 3, "private"),
        Some("keyword.modifier")
    );
    assert_eq!(
        name_of(&spans, text, 3, "readonly"),
        Some("keyword.modifier")
    );
    assert_eq!(
        name_of(&spans, text, 3, "count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 4, "run"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 4, "Service"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 4, "other"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 4, "string"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 5, "method"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 6, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 6, "self"), Some("variable.builtin"));
}

#[test]
fn highlights_php_member_access_and_calls() {
    let text = "\
<?php
$service->run();
$service?->run();
$service::build();
$service::$instance;
$label = $service->label;
$maybe = $service?->label;
$version = Service::VERSION;
";
    let spans = highlight(LanguageId::Php, text, 0, 8);
    assert_eq!(name_of(&spans, text, 1, "$service"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 1, "run"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 2, "run"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 3, "build"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 4, "instance"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "label;"), Some("property"));
    assert_eq!(name_of(&spans, text, 6, "label"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, "Service"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "VERSION"), Some("constant"));
}

#[test]
fn highlights_php_conditions_loops_and_labels() {
    let text = "\
<?php
function walk(array $items): void {
    start:
    foreach ($items as $item) {
        if ($item instanceof Item) {
            goto start;
        }
    }
    while (false) { break; }
}
";
    let spans = highlight(LanguageId::Php, text, 0, 10);
    assert_eq!(name_of(&spans, text, 2, "start"), Some("label"));
    assert_eq!(name_of(&spans, text, 3, "foreach"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 3, "as"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 4, "if"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 4, "instanceof"),
        Some("keyword.operator")
    );
    assert_eq!(name_of(&spans, text, 4, "Item"), Some("type"));
    assert_eq!(name_of(&spans, text, 5, "goto"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 5, "start"), Some("label"));
    assert_eq!(name_of(&spans, text, 8, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 8, "false"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 8, "break"), Some("keyword.repeat"));
}

#[test]
fn highlights_php_property_hooks_and_promotion() {
    let text = "\
<?php
class Widget {
    public private(set) string $label = 'x';
    public string $value {
        get => $this->label;
        set => $value;
    }
    public function __construct(private int $count = 0) {}
}
";
    let spans = highlight(LanguageId::Php, text, 0, 9);
    assert_eq!(name_of(&spans, text, 1, "Widget"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 2, "public"), Some("keyword.modifier"));
    assert_eq!(
        name_of(&spans, text, 2, "private"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 2, "set"), Some("label"));
    assert_eq!(name_of(&spans, text, 2, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "label"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "value"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "get"), Some("label"));
    assert_eq!(name_of(&spans, text, 4, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 4, "label"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "set"), Some("label"));
    assert_eq!(name_of(&spans, text, 7, "__construct"), Some("constructor"));
    assert_eq!(
        name_of(&spans, text, 7, "private"),
        Some("keyword.modifier")
    );
    assert_eq!(
        name_of(&spans, text, 7, "count"),
        Some("variable.parameter")
    );
}

#[test]
fn highlights_php_enum_cases_and_constants() {
    let text = "\
<?php
const LIMIT = 10;
class Limits {
    public const MAX = 20;
}
enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
}
";
    let spans = highlight(LanguageId::Php, text, 0, 9);
    assert_eq!(name_of(&spans, text, 1, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "Limits"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 3, "MAX"), Some("constant"));
    assert_eq!(name_of(&spans, text, 5, "Status"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 5, "string"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 6, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 6, "Active"), Some("constant"));
    assert_eq!(name_of(&spans, text, 7, "Inactive"), Some("constant"));
}

#[test]
fn highlights_php_group_imports() {
    let text = "\
<?php
use Other\\{Thing, OtherThing};
use function Other\\{helper, other as alias};
use const Other\\{FIRST, SECOND as second};
";
    let spans = highlight(LanguageId::Php, text, 0, 4);
    assert_eq!(name_of(&spans, text, 1, "Thing"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "OtherThing"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "helper"), Some("function"));
    assert_eq!(name_of(&spans, text, 2, "other"), Some("function"));
    assert_eq!(name_of(&spans, text, 2, "alias"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "FIRST"), Some("constant"));
    assert_eq!(name_of(&spans, text, 3, "SECOND"), Some("constant"));
    assert_eq!(name_of(&spans, text, 3, "second"), Some("constant"));
}

#[test]
fn highlights_php_array_and_list_builtins() {
    let text = "\
<?php
$array = array(1);
list($first) = [1];
";
    let spans = highlight(LanguageId::Php, text, 0, 3);
    assert_eq!(name_of(&spans, text, 1, "array("), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 2, "list"), Some("function.builtin"));
}
