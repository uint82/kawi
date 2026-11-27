use std::sync::OnceLock;
use std::time::Duration;

use kawi_grammar::{GrammarLoader, LoadedGrammar, Pack};
use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

use super::support::fixture_typescript_pack;

pub const PARSE_BUDGET: Duration = Duration::from_secs(10);

pub const TYPESCRIPT_SOURCE: &str = r#"import { readFile } from "node:fs/promises";
import type { Stats } from "node:fs";

export interface Point<T = number> {
  readonly x: T;
  y: T;
  label?: string;
}

export type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "square"; size: number };

export enum Direction {
  North = "north",
  South = "south",
}

export abstract class Base<T extends object> {
  protected constructor(public readonly value: T) {}
  abstract describe(): string;
}

export class Box<T> extends Base<{ item: T }> {
  #count = 0;
  constructor(item: T) {
    super({ item });
  }
  describe(): string {
    return `box(${this.value.item}) #${this.#count++}`;
  }
}

export async function loadStats(path: string): Promise<Stats> {
  const data = await readFile(path);
  return data.length > 0 ? { size: data.length } : ({} as Stats);
}

export const area = (shape: Shape): number =>
  shape.kind === "circle" ? Math.PI * shape.radius ** 2 : shape.size ** 2;

export function isCircle(
  shape: Shape,
): shape is Extract<Shape, { kind: "circle" }> {
  return shape.kind === "circle";
}

const map = new Map<string, Point<number>>();
map.set("origin", { x: 0, y: 0 });

function logged<T extends (...args: never[]) => unknown>(fn: T): T {
  return ((...args: never[]) => fn(...args)) as T;
}

export class Service {
  @logged
  run(): void {}
}

export default function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}
"#;

pub const ERROR_SOURCE: &str = r#"const value: = 1;
function broken( {
  return `unterminated ${value
}
const pattern = /[/;
"#;

pub const UNICODE_SOURCE: &str = "const café: string = \"é\";\n";

pub const CORPUS: &[(&str, &str)] = &[
    ("features", TYPESCRIPT_SOURCE),
    ("unicode", UNICODE_SOURCE),
    ("errors", ERROR_SOURCE),
    ("empty", ""),
    (
        "crlf",
        "const value: number = 1;\r\nfunction add(a: number, b: number): number {\r\n  return a + b;\r\n}\r\n",
    ),
];

pub const HIGHLIGHTS: &str = include_str!("../../packs/typescript/0.23.2/queries/highlights.scm");
pub const LOCALS: &str = include_str!("../../packs/typescript/0.23.2/queries/locals.scm");
pub const NATIVE_HIGHLIGHTS: &str = concat!(
    include_str!("../../../kawi-syntax/queries/ecmascript.scm"),
    "\n",
    include_str!("../../../kawi-syntax/queries/ecmascript-tokens.scm"),
    "\n",
    include_str!("../../../kawi-syntax/queries/typescript.scm")
);

pub fn grammar() -> &'static LoadedGrammar {
    static GRAMMAR: OnceLock<LoadedGrammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| {
        let pack = Pack::open(fixture_typescript_pack()).expect("open TypeScript pack");
        GrammarLoader::new()
            .expect("create loader")
            .load(&pack)
            .expect("load TypeScript pack")
    })
}

pub fn native_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub fn parse_loaded(grammar: &LoadedGrammar, source: &str) -> Tree {
    let mut parser = grammar.parser().expect("create parser");
    parser.parse(source, None).expect("parse")
}

pub fn captures(
    language: &Language,
    tree: &Tree,
    query_source: &str,
    source: &str,
) -> Vec<(String, usize, usize)> {
    let query = Query::new(language, query_source).expect("compile query");
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut result = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures() {
            result.push((
                names[capture.index as usize].to_string(),
                capture.node.start_byte(),
                capture.node.end_byte(),
            ));
        }
    }
    result
}

pub fn point_at(source: &str, offset: usize) -> Point {
    let mut point = Point::new(0, 0);
    for byte in source.bytes().take(offset) {
        if byte == b'\n' {
            point.row += 1;
            point.column = 0;
        } else {
            point.column += 1;
        }
    }
    point
}

pub fn assert_incremental_parity(source: &str, updated: &str, edit: InputEdit) {
    let grammar = grammar();
    let mut native_parser = Parser::new();
    native_parser
        .set_language(&native_language())
        .expect("set native language");
    let mut wasm_parser = grammar.parser().expect("create wasm parser");

    let mut native_tree = native_parser.parse(source, None).expect("native parse");
    let mut wasm_tree = wasm_parser.parse(source, None).expect("wasm parse");

    native_tree.edit(&edit);
    wasm_tree.edit(&edit);
    let native_after = native_parser
        .parse(updated, Some(&native_tree))
        .expect("native reparse");
    let wasm_after = wasm_parser
        .parse(updated, Some(&wasm_tree))
        .expect("wasm reparse");

    assert_eq!(
        native_after.root_node().to_sexp(),
        wasm_after.root_node().to_sexp()
    );
}
