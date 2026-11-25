use super::*;

#[test]
fn sql_clauses_indent_and_dedent() {
    let mut e = sql_editor("select");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "  ");
    type_text(&mut e, "a, b");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "  ");
    type_text(&mut e, "from t");
    assert_eq!(e.text_lines()[2], "from t");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "where x = 1");
    assert_eq!(e.text_lines()[3], "where x = 1");
}

#[test]
fn sql_case_aligns_branches_and_end() {
    let mut e = sql_editor("SELECT");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    type_text(&mut e, "CASE");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
    type_text(&mut e, "WHEN a THEN");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[3], "      ");
    type_text(&mut e, "1");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "WHEN b THEN");
    assert_eq!(e.text_lines()[4], "    WHEN b THEN");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "2");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END");
    assert_eq!(e.text_lines()[6], "  END");
}

#[test]
fn sql_inline_case_aligns_branches_and_end() {
    let mut e = sql_editor("SELECT\n  x = CASE");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
    type_text(&mut e, "WHEN a THEN 1");
    assert_eq!(e.text_lines()[2], "    WHEN a THEN 1");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "ELSE 2");
    assert_eq!(e.text_lines()[3], "    ELSE 2");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END");
    assert_eq!(e.text_lines()[4], "  END");
}

#[test]
fn sql_nested_blocks_align_end_with_their_opener() {
    let mut e = sql_editor("BEGIN");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    type_text(&mut e, "IF x THEN");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "UPDATE t SET a = 1;");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END IF;");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END;");
    assert_eq!(e.text_lines()[3], "  END IF;");
    assert_eq!(e.text_lines()[4], "END;");
}

#[test]
fn sql_subquery_keeps_clause_indentation() {
    let mut e = sql_editor("SELECT\n  a\nFROM (\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "SELECT");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[4], "    ");
    type_text(&mut e, "b");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[5], "    ");
    type_text(&mut e, "FROM t");
    assert_eq!(e.text_lines()[5], "  FROM t");
}

#[test]
fn sql_values_aligns_with_insert() {
    let mut e = sql_editor("INSERT INTO t (\n  a,\n  b\n)\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "VALUES (");
    assert_eq!(e.text_lines()[4], "VALUES ()");
}

#[test]
fn sql_end_without_opener_keeps_indent() {
    let mut e = sql_editor("SELECT\n  id\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "END");
    assert_eq!(e.text_lines()[2], "  END");
}

#[test]
fn sql_keywords_inside_block_comments_do_not_dedent() {
    let mut e = sql_editor("/*\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "END");
    assert_eq!(e.text_lines()[1], "  END");
}

#[test]
fn sql_dedent_leaves_text_after_the_cursor_alone() {
    let mut e = sql_editor("SELECT\n  id\n  x");
    e.apply(Action::JumpTo(2, 2));
    e.apply(Action::EnterInsert);
    type_text(&mut e, "FROM ");
    assert_eq!(e.text_lines()[2], "  FROM x");
}

#[test]
fn sql_keyword_dedent_is_one_undo_step() {
    let mut e = sql_editor("SELECT\n  id\n  ");
    let before = e.text_lines();
    append_last_line_end(&mut e);
    type_text(&mut e, "FROM");
    e.apply(Action::ExitInsert);
    e.apply(Action::Undo);
    assert_eq!(e.text_lines(), before);
}

#[test]
fn sql_from_after_clause_line_aligns_with_it() {
    let mut e = sql_editor("BEGIN\n  SELECT\n    ");
    append_last_line_end(&mut e);
    type_text(&mut e, "FROM t");
    assert_eq!(e.text_lines()[2], "  FROM t");
}

#[test]
fn sql_else_follows_an_empty_branch() {
    let mut e = sql_editor("CASE\n  WHEN a THEN\n    ");
    append_last_line_end(&mut e);
    type_text(&mut e, "ELSE");
    assert_eq!(e.text_lines()[2], "  ELSE");
}

#[test]
fn sql_merge_when_aligns_with_previous_when() {
    let mut e = sql_editor(
        "MERGE INTO t USING s ON t.id = s.id\nWHEN MATCHED THEN\n  UPDATE SET a = 1\n  ",
    );
    append_last_line_end(&mut e);
    type_text(&mut e, "WHEN NOT MATCHED THEN");
    assert_eq!(e.text_lines()[3], "WHEN NOT MATCHED THEN");
}

#[test]
fn sql_if_branches_align_with_if() {
    let mut e = sql_editor("BEGIN\n  IF x THEN\n    a;\n    ");
    append_last_line_end(&mut e);
    type_text(&mut e, "ELSE");
    assert_eq!(e.text_lines()[3], "  ELSE");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[4], "    ");
    type_text(&mut e, "b;");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END IF;");
    assert_eq!(e.text_lines()[5], "  END IF;");
}

#[test]
fn sql_exception_handlers_indent_under_exception() {
    let mut e = sql_editor("BEGIN\n  a;\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "EXCEPTION");
    assert_eq!(e.text_lines()[2], "EXCEPTION");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[3], "  ");
    type_text(&mut e, "WHEN OTHERS THEN");
    assert_eq!(e.text_lines()[3], "  WHEN OTHERS THEN");
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[4], "    ");
    type_text(&mut e, "NULL;");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END;");
    assert_eq!(e.text_lines()[5], "END;");
}

#[test]
fn sql_repeat_until_loop_aligns_with_repeat() {
    let mut e = sql_editor("REPEAT\n  a;\n  ");
    append_last_line_end(&mut e);
    type_text(&mut e, "UNTIL done");
    assert_eq!(e.text_lines()[2], "UNTIL done");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END REPEAT;");
    assert_eq!(e.text_lines()[3], "END REPEAT;");
}

#[test]
fn sql_while_do_loop_indents_its_body() {
    let mut e = sql_editor("WHILE x < 10 DO");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "  ");
    type_text(&mut e, "SET x = x + 1;");
    e.apply(Action::InsertNewline);
    type_text(&mut e, "END WHILE;");
    assert_eq!(e.text_lines()[2], "END WHILE;");
}
