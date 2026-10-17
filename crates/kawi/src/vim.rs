use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::keymap::{has_ctrl_or_alt, normalize_interrupt, Keymap};
use kawi_editor::Action;
use kawi_editor::CaseMotion;
use kawi_editor::Mode;
use kawi_editor::Object;
use kawi_editor::ShiftMotion;

fn is_redo_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('r'))
}

fn insert_control_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T') => Some(Action::InsertShiftRight),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(Action::InsertShiftLeft),
        _ => None,
    }
}

fn jump_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('o') | KeyCode::Char('O') => Some(Action::JumpOlder),
        KeyCode::Char('i') | KeyCode::Char('I') => Some(Action::JumpNewer),
        _ => None,
    }
}

fn scroll_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('u') | KeyCode::Char('U') => Some(Action::ScrollHalfUp),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(Action::ScrollHalfDown),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(Action::ScrollPageDown),
        KeyCode::Char('b') | KeyCode::Char('B') => Some(Action::ScrollPageUp),
        _ => None,
    }
}

fn place_action(pushed: char) -> Option<Action> {
    match pushed {
        'z' => Some(Action::PlaceCenter),
        't' => Some(Action::PlaceTop),
        'b' => Some(Action::PlaceBottom),
        _ => None,
    }
}

const OPERATOR_SEQUENCES: &[(&str, Action)] = &[
    ("dd", Action::DeleteLine),
    ("dw", Action::DeleteWord),
    ("db", Action::DeleteWordBackward),
    ("de", Action::DeleteWordEnd),
    ("d0", Action::DeleteToStart),
    ("d^", Action::DeleteToFirstNonBlank),
    ("d$", Action::DeleteToEnd),
    ("d%", Action::DeleteToMatch),
    ("d(", Action::DeleteToPrevSentence),
    ("d)", Action::DeleteToNextSentence),
    ("d{", Action::DeleteToPrevParagraph),
    ("d}", Action::DeleteToNextParagraph),
    ("dG", Action::DeleteToLastLine),
    ("dgg", Action::DeleteToFirstLine),
    ("dH", Action::DeleteToViewportTop),
    ("dM", Action::DeleteToViewportMiddle),
    ("dL", Action::DeleteToViewportBottom),
    ("dge", Action::DeleteWordEndBackward),
    (">>", Action::ShiftRight),
    ("<<", Action::ShiftLeft),
    ("==", Action::Reindent),
    ("diw", Action::DeleteInnerWord),
    ("daw", Action::DeleteAroundWord),
    ("cc", Action::ChangeLine),
    ("cw", Action::ChangeWord),
    ("ce", Action::ChangeWordEnd),
    ("c0", Action::ChangeToStart),
    ("c^", Action::ChangeToFirstNonBlank),
    ("c$", Action::ChangeToEnd),
    ("c%", Action::ChangeToMatch),
    ("c(", Action::ChangeToPrevSentence),
    ("c)", Action::ChangeToNextSentence),
    ("c{", Action::ChangeToPrevParagraph),
    ("c}", Action::ChangeToNextParagraph),
    ("cG", Action::ChangeToLastLine),
    ("cgg", Action::ChangeToFirstLine),
    ("cH", Action::ChangeToViewportTop),
    ("cM", Action::ChangeToViewportMiddle),
    ("cL", Action::ChangeToViewportBottom),
    ("cge", Action::ChangeWordEndBackward),
    ("ciw", Action::ChangeInnerWord),
    ("caw", Action::ChangeAroundWord),
    ("yy", Action::YankLine),
    ("y0", Action::YankToStart),
    ("y^", Action::YankToFirstNonBlank),
    ("y$", Action::YankToEnd),
    ("y%", Action::YankToMatch),
    ("y(", Action::YankToPrevSentence),
    ("y)", Action::YankToNextSentence),
    ("y{", Action::YankToPrevParagraph),
    ("y}", Action::YankToNextParagraph),
    ("yG", Action::YankToLastLine),
    ("ygg", Action::YankToFirstLine),
    ("yH", Action::YankToViewportTop),
    ("yM", Action::YankToViewportMiddle),
    ("yL", Action::YankToViewportBottom),
    ("yge", Action::YankWordEndBackward),
    ("yiw", Action::YankInnerWord),
    ("yaw", Action::YankAroundWord),
];

fn sequence_action(sequence: &str) -> Option<Action> {
    let found = OPERATOR_SEQUENCES
        .iter()
        .find(|(keys, _)| *keys == sequence);
    found.map(|(_, action)| *action)
}

fn is_sequence_prefix(sequence: &str) -> bool {
    OPERATOR_SEQUENCES
        .iter()
        .any(|(keys, _)| *keys != sequence && keys.starts_with(sequence))
}

fn arms_pending(pushed: char) -> bool {
    matches!(
        pushed,
        'd' | 'c' | 'y' | '>' | '<' | '=' | 'f' | 'F' | 't' | 'T' | 'g' | 'z'
    )
}

fn is_visual_motion_key(pushed: char) -> bool {
    matches!(
        pushed,
        'h' | 'j' | 'k' | 'l' | 'w' | 'b' | 'e' | '$' | '^' | '%' | '(' | ')' | '{' | '}'
    )
}

fn display_motion_action(pushed: char) -> Option<Action> {
    match pushed {
        'j' => Some(Action::MoveDown),
        'k' => Some(Action::MoveUp),
        '0' => Some(Action::MoveToLineStart),
        '$' => Some(Action::MoveToLineEnd),
        _ => None,
    }
}

fn object_action(sequence: &str) -> Option<Action> {
    let mut chars = sequence.chars();
    let operator = chars.next()?;
    let scope = chars.next()?;
    let target = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if !matches!(operator, 'd' | 'c' | 'y') || !matches!(scope, 'i' | 'a') {
        return None;
    }
    let object = object_target(target)?;
    let inner = scope == 'i';
    Some(match operator {
        'd' => Action::DeleteObject { inner, object },
        'c' => Action::ChangeObject { inner, object },
        _ => Action::YankObject { inner, object },
    })
}

fn object_target(target: char) -> Option<Object> {
    match target {
        '(' | ')' => Some(Object::Paren),
        '{' | '}' => Some(Object::Brace),
        '[' | ']' => Some(Object::Bracket),
        '<' | '>' => Some(Object::Angle),
        '\'' => Some(Object::SingleQuote),
        '"' => Some(Object::DoubleQuote),
        '`' => Some(Object::Backtick),
        'p' => Some(Object::Paragraph),
        _ => None,
    }
}

fn trailing_operator(pending: &str) -> Option<char> {
    let operator = pending.trim_start_matches(|ch: char| ch.is_ascii_digit());
    if operator.len() == 1 && matches!(operator, "d" | "c" | "y") {
        return operator.chars().next();
    }
    None
}

#[derive(Clone, Copy)]
struct FindMemory {
    target: char,
    forward: bool,
    till: bool,
}

pub struct VimKeymap {
    pending: String,
    last_find: Option<FindMemory>,
}

impl VimKeymap {
    pub fn new() -> Self {
        Self {
            pending: String::new(),
            last_find: None,
        }
    }

    fn map_insert(&mut self, key: KeyEvent) -> Action {
        self.clear_pending();
        if let Some(action) = insert_control_action(key) {
            return action;
        }
        match key.code {
            KeyCode::Esc => Action::ExitInsert,
            KeyCode::Enter => Action::InsertNewline,
            KeyCode::Tab => Action::InsertTab,
            KeyCode::Backspace => Action::DeleteBackward,
            KeyCode::Delete => Action::DeleteForward,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Char(c) => self.map_insert_char(key, c),
            _ => Action::Noop,
        }
    }

    fn map_insert_char(&mut self, key: KeyEvent, c: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::InsertChar(c)
    }

    fn map_normal(&mut self, key: KeyEvent) -> Action {
        if let Some(action) = jump_action(key) {
            self.clear_pending();
            return action;
        }
        if let Some(action) = scroll_action(key) {
            self.clear_pending();
            return action;
        }
        if !self.pending.is_empty() {
            return self.resolve_pending(key);
        }
        self.map_normal_fresh(key)
    }

    fn map_normal_fresh(&mut self, key: KeyEvent) -> Action {
        if is_redo_key(key) {
            return Action::Redo;
        }
        if let KeyCode::Char(c) = key.code {
            if matches!(c, '1'..='9') && !has_ctrl_or_alt(key) {
                return self.push_digit(c);
            }
        }
        match key.code {
            KeyCode::Esc => Action::Noop,
            KeyCode::Enter => Action::MoveToNextLineStart,
            KeyCode::Tab => Action::JumpNewer,
            KeyCode::Backspace => Action::MoveLeft,
            KeyCode::Delete => Action::DeleteForward,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Char('h') => self.map_plain_or_noop(key, Action::MoveLeft),
            KeyCode::Char('j') => self.map_plain_or_noop(key, Action::MoveDown),
            KeyCode::Char('k') => self.map_plain_or_noop(key, Action::MoveUp),
            KeyCode::Char('l') => self.map_plain_or_noop(key, Action::MoveRight),
            KeyCode::Char('x') => self.map_plain_or_noop(key, Action::DeleteForward),
            KeyCode::Char('0') => self.map_plain_or_noop(key, Action::MoveToLineStart),
            KeyCode::Char('^') => self.map_plain_or_noop(key, Action::MoveToFirstNonBlank),
            KeyCode::Char('%') => self.map_plain_or_noop(key, Action::MoveToMatch),
            KeyCode::Char('(') => self.map_plain_or_noop(key, Action::MoveToPrevSentence),
            KeyCode::Char(')') => self.map_plain_or_noop(key, Action::MoveToNextSentence),
            KeyCode::Char('{') => self.map_plain_or_noop(key, Action::MoveToPrevParagraph),
            KeyCode::Char('}') => self.map_plain_or_noop(key, Action::MoveToNextParagraph),
            KeyCode::Char('$') => self.map_plain_or_noop(key, Action::MoveToLineEnd),
            KeyCode::Char('+') => self.map_plain_or_noop(key, Action::MoveToNextLineStart),
            KeyCode::Char('-') => self.map_plain_or_noop(key, Action::MoveToPrevLineStart),
            KeyCode::Char('i') => self.map_plain_or_noop(key, Action::EnterInsert),
            KeyCode::Char('a') => self.map_plain_or_noop(key, Action::Append),
            KeyCode::Char('A') => self.map_plain_or_noop(key, Action::AppendLineEnd),
            KeyCode::Char('I') => self.map_plain_or_noop(key, Action::InsertLineStart),
            KeyCode::Char('u') => self.map_plain_or_noop(key, Action::Undo),
            KeyCode::Char('~') => self.map_plain_or_noop(key, Action::ToggleChar),
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::MoveToNextWordStart),
            KeyCode::Char('b') => self.map_plain_or_noop(key, Action::MoveToPrevWordStart),
            KeyCode::Char('e') => self.map_plain_or_noop(key, Action::MoveToNextWordEnd),
            KeyCode::Char('o') => self.map_plain_or_noop(key, Action::OpenLineBelow),
            KeyCode::Char('p') => self.map_plain_or_noop(key, Action::PasteAfter),
            KeyCode::Char('P') => self.map_plain_or_noop(key, Action::PasteBefore),
            KeyCode::Char('n') => self.map_plain_or_noop(key, Action::NextMatch),
            KeyCode::Char('N') => self.map_plain_or_noop(key, Action::PrevMatch),
            KeyCode::Char('/') => self.map_plain_or_noop(key, Action::EnterSearch),
            KeyCode::Char('D') => self.map_plain_or_noop(key, Action::DeleteToEnd),
            KeyCode::Char('C') => self.map_plain_or_noop(key, Action::ChangeToEnd),
            KeyCode::Char('Y') => self.map_plain_or_noop(key, Action::YankToEnd),
            KeyCode::Char('f') => self.arm_find(key, 'f'),
            KeyCode::Char('F') => self.arm_find(key, 'F'),
            KeyCode::Char('t') => self.arm_find(key, 't'),
            KeyCode::Char('T') => self.arm_find(key, 'T'),
            KeyCode::Char(';') => self.repeat_motion(key, false),
            KeyCode::Char(',') => self.repeat_motion(key, true),
            KeyCode::Char('d') => self.arm_operator(key, 'd'),
            KeyCode::Char('c') => self.arm_operator(key, 'c'),
            KeyCode::Char('y') => self.arm_operator(key, 'y'),
            KeyCode::Char('.') => self.map_plain_or_noop(key, Action::RepeatLastChange),
            KeyCode::Char('>') => self.arm_operator(key, '>'),
            KeyCode::Char('<') => self.arm_operator(key, '<'),
            KeyCode::Char('=') => self.arm_operator(key, '='),
            KeyCode::Char('g') => self.arm_case_operator(key),
            KeyCode::Char('z') => self.arm_place(key),
            KeyCode::Char('H') => self.map_plain_or_noop(key, Action::MoveToViewportTop),
            KeyCode::Char('M') => self.map_plain_or_noop(key, Action::MoveToViewportMiddle),
            KeyCode::Char('L') => self.map_plain_or_noop(key, Action::MoveToViewportBottom),
            KeyCode::Char('G') => self.map_plain_or_noop(key, Action::GotoLastLine),
            KeyCode::Char(':') => self.arm_command(key),
            KeyCode::Char('v') => self.map_plain_or_noop(key, Action::EnterVisual),
            KeyCode::Char('V') => self.map_plain_or_noop(key, Action::EnterVisualLine),
            _ => Action::Noop,
        }
    }

    fn map_plain_or_noop(&mut self, key: KeyEvent, action: Action) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        action
    }

    fn clear_pending(&mut self) {
        self.pending.clear();
    }

    fn pending_key(&self) -> Option<&str> {
        if self.pending.is_empty() {
            return None;
        }
        Some(&self.pending)
    }

    fn arm_operator(&mut self, key: KeyEvent, operator: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push(operator);
        Action::BeginSequence
    }

    fn arm_case_operator(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push('g');
        Action::Noop
    }

    fn arm_place(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push('z');
        Action::Noop
    }

    fn z_prefix_armed(&self) -> bool {
        self.pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .starts_with('z')
    }

    fn resolve_z_pending(&mut self, pushed: char) -> Action {
        let body: String = self
            .pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .to_string();
        let Some(tail) = body.strip_prefix('z') else {
            return self.cancel_sequence();
        };
        if !tail.chars().all(|ch| ch.is_ascii_digit()) {
            return self.cancel_sequence();
        }
        if pushed.is_ascii_digit() {
            return self.push_digit(pushed);
        }
        let Some(action) = place_action(pushed) else {
            return self.cancel_sequence();
        };
        self.clear_pending();
        action
    }

    fn case_prefix_armed(&self) -> bool {
        self.pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .starts_with('g')
    }

    fn resolve_case_pending(&mut self, pushed: char) -> Action {
        let body: String = self
            .pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .to_string();
        if let Some(rest) = body.strip_prefix("g~") {
            return self.complete_case_operator("g~", rest, pushed);
        }
        if let Some(rest) = body.strip_prefix("gu") {
            return self.complete_case_operator("gu", rest, pushed);
        }
        if let Some(rest) = body.strip_prefix("gU") {
            return self.complete_case_operator("gU", rest, pushed);
        }
        self.complete_g_prefix(pushed, &body)
    }

    fn complete_g_prefix(&mut self, pushed: char, body: &str) -> Action {
        if body == "g" && matches!(pushed, '~' | 'u' | 'U') {
            self.pending.push(pushed);
            return Action::BeginSequence;
        }
        if body == "g" && pushed == 'g' {
            self.clear_pending();
            return Action::GotoFirstLine;
        }
        if body == "g" && pushed == 'v' {
            self.clear_pending();
            return Action::ReselectVisual;
        }
        if body == "g" && pushed == 'i' {
            self.clear_pending();
            return Action::InsertAtLast;
        }
        if body == "g" && pushed == 'I' {
            self.clear_pending();
            return Action::InsertAtColumnStart;
        }
        if body == "g" && pushed == 'e' {
            self.clear_pending();
            return Action::MoveToPrevWordEnd;
        }
        if body == "g" {
            if let Some(action) = display_motion_action(pushed) {
                self.clear_pending();
                return action;
            }
        }
        self.cancel_sequence()
    }

    fn complete_case_operator(&mut self, operator: &str, rest: &str, pushed: char) -> Action {
        if rest.is_empty() || rest.chars().all(|ch| ch.is_ascii_digit()) {
            if (pushed == '0' && rest.is_empty()) || !pushed.is_ascii_digit() {
                return self.complete_case_motion(operator, pushed);
            }
            return self.push_digit(pushed);
        }
        match rest {
            "f" | "F" | "t" | "T" => self.complete_case_find(operator, rest, pushed),
            "i" | "a" => self.complete_case_object(operator, rest, pushed),
            "g" => self.complete_case_g_motion(operator, pushed),
            _ => self.cancel_sequence(),
        }
    }

    fn complete_case_g_motion(&mut self, operator: &str, pushed: char) -> Action {
        let motion = match pushed {
            'g' => CaseMotion::GotoLine { last: false },
            'e' => CaseMotion::WordEndBackward,
            _ => return self.cancel_sequence(),
        };
        self.clear_pending();
        self.case_motion_action(operator, motion)
    }

    fn shift_prefix_armed(&self) -> bool {
        let body: String = self
            .pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .to_string();
        body.starts_with('>') || body.starts_with('<') || body.starts_with('=')
    }

    fn resolve_shift_pending(&mut self, pushed: char) -> Action {
        let candidate = format!("{}{}", self.stripped_pending(), pushed);
        if sequence_action(&candidate).is_some() || is_sequence_prefix(&candidate) {
            return self.extend_sequence(pushed);
        }
        let body: String = self
            .pending
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .to_string();
        let operator = body.chars().next().unwrap_or('>');
        let rest = body.get(1..).unwrap_or("");
        self.complete_shift_operator(operator, rest, pushed)
    }

    fn complete_shift_operator(&mut self, operator: char, rest: &str, pushed: char) -> Action {
        if rest.is_empty() || rest.chars().all(|ch| ch.is_ascii_digit()) {
            if (pushed == '0' && rest.is_empty()) || !pushed.is_ascii_digit() {
                return self.complete_shift_motion(operator, pushed);
            }
            return self.push_digit(pushed);
        }
        match rest {
            "f" | "F" | "t" | "T" => self.complete_shift_find(operator, rest, pushed),
            "i" | "a" => self.complete_shift_object(operator, rest, pushed),
            "g" => self.complete_shift_g_motion(operator, pushed),
            _ => self.cancel_sequence(),
        }
    }

    fn complete_shift_motion(&mut self, operator: char, pushed: char) -> Action {
        if matches!(pushed, ';' | ',') {
            return self.repeat_shift_motion(operator, pushed == ',');
        }
        let motion = match pushed {
            'w' => ShiftMotion::WordForward,
            'b' => ShiftMotion::WordBackward,
            'e' => ShiftMotion::WordEnd,
            '$' => ShiftMotion::LineEnd,
            '0' => ShiftMotion::LineStart,
            'j' => ShiftMotion::Lines { upward: false },
            'k' => ShiftMotion::Lines { upward: true },
            'G' => ShiftMotion::GotoLine { last: true },
            'g' => {
                self.pending.push(pushed);
                return Action::Noop;
            }
            'f' | 'F' | 't' | 'T' | 'i' | 'a' => {
                self.pending.push(pushed);
                return if matches!(pushed, 'i' | 'a') {
                    Action::Noop
                } else {
                    Action::BeginSequence
                };
            }
            _ => return self.cancel_sequence(),
        };
        self.clear_pending();
        self.shift_motion_action(operator, motion)
    }

    fn shift_motion_action(&self, operator: char, motion: ShiftMotion) -> Action {
        match operator {
            '>' => Action::ShiftRightAt { motion },
            '<' => Action::ShiftLeftAt { motion },
            _ => Action::ReindentAt { motion },
        }
    }

    fn repeat_shift_motion(&mut self, operator: char, reverse: bool) -> Action {
        let Some(found) = self.last_find else {
            return self.cancel_sequence();
        };
        let motion = ShiftMotion::RepeatFind {
            target: found.target,
            forward: found.forward != reverse,
            till: found.till,
        };
        self.clear_pending();
        self.shift_motion_action(operator, motion)
    }

    fn complete_shift_find(&mut self, operator: char, find: &str, target: char) -> Action {
        let forward = matches!(find, "f" | "t");
        let till = matches!(find, "t" | "T");
        self.store_find(target, forward, till);
        let motion = ShiftMotion::Find {
            target,
            forward,
            till,
        };
        self.clear_pending();
        self.shift_motion_action(operator, motion)
    }

    fn complete_shift_object(&mut self, operator: char, scope: &str, pushed: char) -> Action {
        let Some(object) = object_target(pushed) else {
            return self.cancel_sequence();
        };
        let motion = ShiftMotion::Object {
            inner: scope == "i",
            object,
        };
        self.clear_pending();
        self.shift_motion_action(operator, motion)
    }

    fn complete_shift_g_motion(&mut self, operator: char, pushed: char) -> Action {
        let motion = match pushed {
            'g' => ShiftMotion::GotoLine { last: false },
            'e' => ShiftMotion::WordEndBackward,
            _ => return self.cancel_sequence(),
        };
        self.clear_pending();
        self.shift_motion_action(operator, motion)
    }

    fn complete_case_motion(&mut self, operator: &str, pushed: char) -> Action {
        if matches!(pushed, ';' | ',') {
            return self.repeat_case_motion(operator, pushed == ',');
        }
        let motion = match pushed {
            'w' => CaseMotion::WordForward,
            'b' => CaseMotion::WordBackward,
            'e' => CaseMotion::WordEnd,
            '$' => CaseMotion::LineEnd,
            '0' => CaseMotion::LineStart,
            'j' => CaseMotion::Lines { upward: false },
            'k' => CaseMotion::Lines { upward: true },
            'G' => CaseMotion::GotoLine { last: true },
            'g' => {
                self.pending.push(pushed);
                return Action::Noop;
            }
            'f' | 'F' | 't' | 'T' | 'i' | 'a' => {
                self.pending.push(pushed);
                return if matches!(pushed, 'i' | 'a') {
                    Action::Noop
                } else {
                    Action::BeginSequence
                };
            }
            '~' | 'u' | 'U' if Some(pushed) == operator.chars().nth(1) => CaseMotion::OperatorLines,
            _ => return self.cancel_sequence(),
        };
        self.clear_pending();
        self.case_motion_action(operator, motion)
    }

    fn case_motion_action(&self, operator: &str, motion: CaseMotion) -> Action {
        match operator {
            "g~" => Action::ToggleCaseAt { motion },
            "gu" => Action::LowercaseAt { motion },
            _ => Action::UppercaseAt { motion },
        }
    }

    fn repeat_case_motion(&mut self, operator: &str, reverse: bool) -> Action {
        let Some(found) = self.last_find else {
            return self.cancel_sequence();
        };
        let motion = CaseMotion::RepeatFind {
            target: found.target,
            forward: found.forward != reverse,
            till: found.till,
        };
        self.clear_pending();
        self.case_motion_action(operator, motion)
    }

    fn complete_case_find(&mut self, operator: &str, find: &str, target: char) -> Action {
        let forward = matches!(find, "f" | "t");
        let till = matches!(find, "t" | "T");
        self.store_find(target, forward, till);
        let motion = CaseMotion::Find {
            target,
            forward,
            till,
        };
        self.clear_pending();
        self.case_motion_action(operator, motion)
    }

    fn complete_case_object(&mut self, operator: &str, scope: &str, pushed: char) -> Action {
        let Some(object) = object_target(pushed) else {
            return self.cancel_sequence();
        };
        let motion = CaseMotion::Object {
            inner: scope == "i",
            object,
        };
        self.clear_pending();
        self.case_motion_action(operator, motion)
    }

    fn arm_find(&mut self, key: KeyEvent, find: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push(find);
        Action::BeginSequence
    }

    fn repeat_motion(&mut self, key: KeyEvent, reverse: bool) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        match self.last_find {
            Some(found) => Action::RepeatFind {
                target: found.target,
                forward: found.forward != reverse,
                till: found.till,
            },
            None => Action::Noop,
        }
    }

    fn repeat_pending(&mut self, reverse: bool) -> Action {
        let Some(remembered) = self.last_find else {
            return self.cancel_sequence();
        };
        let forward = remembered.forward != reverse;
        let stripped = self.stripped_pending();
        let action = match stripped.as_str() {
            "" => Action::RepeatFind {
                target: remembered.target,
                forward,
                till: remembered.till,
            },
            "d" => Action::DeleteRepeatFind {
                target: remembered.target,
                forward,
                till: remembered.till,
            },
            "c" => Action::ChangeRepeatFind {
                target: remembered.target,
                forward,
                till: remembered.till,
            },
            "y" => Action::YankRepeatFind {
                target: remembered.target,
                forward,
                till: remembered.till,
            },
            ">" => Action::ShiftRightAt {
                motion: ShiftMotion::RepeatFind {
                    target: remembered.target,
                    forward,
                    till: remembered.till,
                },
            },
            "<" => Action::ShiftLeftAt {
                motion: ShiftMotion::RepeatFind {
                    target: remembered.target,
                    forward,
                    till: remembered.till,
                },
            },
            "=" => Action::ReindentAt {
                motion: ShiftMotion::RepeatFind {
                    target: remembered.target,
                    forward,
                    till: remembered.till,
                },
            },
            _ => return self.cancel_sequence(),
        };
        self.clear_pending();
        action
    }

    fn arm_command(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::EnterCommand
    }

    fn resolve_pending(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(c) if !has_ctrl_or_alt(key) => {
                if self.case_prefix_armed() {
                    return self.resolve_case_pending(c);
                }
                if self.z_prefix_armed() {
                    return self.resolve_z_pending(c);
                }
                if self.pending_find().is_some() {
                    return self.complete_find(c);
                }
                if self.shift_prefix_armed() {
                    return self.resolve_shift_pending(c);
                }
                match c {
                    ';' => self.repeat_pending(false),
                    ',' => self.repeat_pending(true),
                    '0' => self.zero_key(),
                    _ => self.sequence_key(c),
                }
            }
            _ => self.cancel_sequence(),
        }
    }

    fn sequence_key(&mut self, pushed: char) -> Action {
        if matches!(pushed, '1'..='9') {
            return self.push_digit(pushed);
        }
        if matches!(pushed, 'f' | 'F' | 't' | 'T') && self.operator_armed() {
            self.pending.push(pushed);
            return Action::BeginSequence;
        }
        self.extend_sequence(pushed)
    }

    fn pending_find(&self) -> Option<(Option<char>, char)> {
        let mut chars = self.pending.chars().peekable();
        while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            chars.next();
        }
        let operator = match chars.peek() {
            Some('d') | Some('c') | Some('y') => chars.next(),
            _ => None,
        };
        while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            chars.next();
        }
        match (chars.next(), chars.next()) {
            (Some(find), None) if matches!(find, 'f' | 'F' | 't' | 'T') => Some((operator, find)),
            _ => None,
        }
    }

    fn complete_find(&mut self, target: char) -> Action {
        let Some((operator, find)) = self.pending_find() else {
            return self.cancel_sequence();
        };
        let forward = matches!(find, 'f' | 't');
        let till = matches!(find, 't' | 'T');
        self.store_find(target, forward, till);
        let action = match operator {
            Some('d') => Action::DeleteFind {
                target,
                forward,
                till,
            },
            Some('c') => Action::ChangeFind {
                target,
                forward,
                till,
            },
            Some('y') => Action::YankFind {
                target,
                forward,
                till,
            },
            _ => Action::FindChar {
                target,
                forward,
                till,
            },
        };
        self.clear_pending();
        action
    }

    fn operator_armed(&self) -> bool {
        matches!(self.stripped_pending().as_str(), "d" | "c" | "y")
    }

    fn store_find(&mut self, target: char, forward: bool, till: bool) {
        self.last_find = Some(FindMemory {
            target,
            forward,
            till,
        });
    }

    fn stripped_pending(&self) -> String {
        self.pending
            .chars()
            .filter(|ch| !ch.is_ascii_digit())
            .collect()
    }

    fn zero_key(&mut self) -> Action {
        if self.pending.is_empty() {
            return Action::MoveToLineStart;
        }
        if trailing_operator(&self.pending).is_some() {
            self.pending.push('0');
            return self.resolve_sequence();
        }
        self.push_digit('0')
    }

    fn push_digit(&mut self, digit: char) -> Action {
        self.pending.push(digit);
        Action::PushCount(digit.to_digit(10).unwrap_or(0) as u8)
    }

    fn extend_sequence(&mut self, pushed: char) -> Action {
        if self.only_digits_pending() {
            if arms_pending(pushed) {
                self.pending.push(pushed);
                return Action::BeginSequence;
            }
            self.clear_pending();
            return self.map_normal_fresh(KeyEvent::new(KeyCode::Char(pushed), KeyModifiers::NONE));
        }
        self.pending.push(pushed);
        self.resolve_sequence()
    }

    fn only_digits_pending(&self) -> bool {
        !self.pending.is_empty() && self.pending.chars().all(|ch| ch.is_ascii_digit())
    }

    fn resolve_sequence(&mut self) -> Action {
        if let Some(action) = sequence_action(&self.pending) {
            return self.complete_sequence(action);
        }
        let stripped = self.stripped_pending();
        if let Some(action) = sequence_action(&stripped) {
            return self.complete_sequence(action);
        }
        if let Some(action) = object_action(&stripped) {
            return self.complete_sequence(action);
        }
        if is_sequence_prefix(&stripped) {
            return Action::Noop;
        }
        self.cancel_sequence()
    }

    fn complete_sequence(&mut self, action: Action) -> Action {
        self.clear_pending();
        action
    }

    fn cancel_sequence(&mut self) -> Action {
        self.clear_pending();
        Action::CancelPending
    }

    fn map_command(&mut self, key: KeyEvent) -> Action {
        crate::keymap::map_command_mode(key)
    }

    fn map_visual_mode(&mut self, key: KeyEvent, mode: Mode) -> Action {
        if let Some(action) = scroll_action(key) {
            self.clear_pending();
            return action;
        }
        if !self.pending.is_empty() {
            return self.resolve_visual_pending(key, mode);
        }
        self.map_visual_fresh(key, mode)
    }

    fn map_visual_fresh(&mut self, key: KeyEvent, mode: Mode) -> Action {
        match key.code {
            KeyCode::Esc => Action::CancelVisual,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Char('h') => self.map_plain_or_noop(key, Action::MoveLeft),
            KeyCode::Char('j') => self.map_plain_or_noop(key, Action::MoveDown),
            KeyCode::Char('k') => self.map_plain_or_noop(key, Action::MoveUp),
            KeyCode::Char('l') => self.map_plain_or_noop(key, Action::MoveRight),
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::MoveToNextWordStart),
            KeyCode::Char('b') => self.map_plain_or_noop(key, Action::MoveToPrevWordStart),
            KeyCode::Char('e') => self.map_plain_or_noop(key, Action::MoveToNextWordEnd),
            KeyCode::Char('$') => self.map_plain_or_noop(key, Action::MoveToLineEnd),
            KeyCode::Char('0') => self.map_plain_or_noop(key, Action::MoveToLineStart),
            KeyCode::Char('^') => self.map_plain_or_noop(key, Action::MoveToFirstNonBlank),
            KeyCode::Char('%') => self.map_plain_or_noop(key, Action::MoveToMatch),
            KeyCode::Char('(') => self.map_plain_or_noop(key, Action::MoveToPrevSentence),
            KeyCode::Char(')') => self.map_plain_or_noop(key, Action::MoveToNextSentence),
            KeyCode::Char('{') => self.map_plain_or_noop(key, Action::MoveToPrevParagraph),
            KeyCode::Char('}') => self.map_plain_or_noop(key, Action::MoveToNextParagraph),
            KeyCode::Char('d') => self.map_plain_or_noop(key, Action::DeleteSelection),
            KeyCode::Char('y') => self.map_plain_or_noop(key, Action::YankSelection),
            KeyCode::Char('p') => self.map_plain_or_noop(key, Action::PasteOverSelection),
            KeyCode::Char('P') => self.map_plain_or_noop(key, Action::PasteOverSelection),
            KeyCode::Char('c') => self.map_plain_or_noop(key, Action::ChangeSelection),
            KeyCode::Char('s') => self.map_plain_or_noop(key, Action::ChangeSelection),
            KeyCode::Char('D') => self.map_plain_or_noop(key, Action::DeleteLines),
            KeyCode::Char('Y') => self.map_plain_or_noop(key, Action::YankLines),
            KeyCode::Char('C') => self.map_plain_or_noop(key, Action::ChangeLines),
            KeyCode::Char('S') => self.map_plain_or_noop(key, Action::ChangeLines),
            KeyCode::Char('>') => self.map_plain_or_noop(key, Action::ShiftRight),
            KeyCode::Char('<') => self.map_plain_or_noop(key, Action::ShiftLeft),
            KeyCode::Char('=') => self.map_plain_or_noop(key, Action::Reindent),
            KeyCode::Char('~') => self.map_plain_or_noop(key, Action::ToggleCase),
            KeyCode::Char('u') => self.map_plain_or_noop(key, Action::Lowercase),
            KeyCode::Char('U') => self.map_plain_or_noop(key, Action::Uppercase),
            KeyCode::Char('g') => self.arm_visual_g(key),
            KeyCode::Char('z') => self.arm_visual_z(key),
            KeyCode::Char('H') => self.map_plain_or_noop(key, Action::MoveToViewportTop),
            KeyCode::Char('M') => self.map_plain_or_noop(key, Action::MoveToViewportMiddle),
            KeyCode::Char('L') => self.map_plain_or_noop(key, Action::MoveToViewportBottom),
            KeyCode::Char('G') => self.map_plain_or_noop(key, Action::GotoLastLine),
            KeyCode::Char('o') => self.map_plain_or_noop(key, Action::SwapSelectionEnds),
            KeyCode::Char('O') => self.map_plain_or_noop(key, Action::SwapSelectionEnds),
            KeyCode::Char('v') => self.map_plain_or_noop(key, self.visual_v_action(mode)),
            KeyCode::Char('V') => self.map_plain_or_noop(key, self.visual_big_v_action(mode)),
            KeyCode::Char('i') => self.arm_visual_object(key),
            KeyCode::Char('a') => self.arm_visual_object(key),
            KeyCode::Char('f') => self.arm_visual_find(key, 'f'),
            KeyCode::Char('F') => self.arm_visual_find(key, 'F'),
            KeyCode::Char('t') => self.arm_visual_find(key, 't'),
            KeyCode::Char('T') => self.arm_visual_find(key, 'T'),
            KeyCode::Char(';') => self.repeat_motion(key, false),
            KeyCode::Char(',') => self.repeat_motion(key, true),
            KeyCode::Char(c) if c.is_ascii_digit() => self.push_visual_digit(key, c),
            _ => Action::Noop,
        }
    }

    fn visual_v_action(&self, mode: Mode) -> Action {
        match mode {
            Mode::VisualLine => Action::EnterVisual,
            _ => Action::CancelVisual,
        }
    }

    fn visual_big_v_action(&self, mode: Mode) -> Action {
        match mode {
            Mode::Visual => Action::EnterVisualLine,
            _ => Action::CancelVisual,
        }
    }

    fn arm_visual_object(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push(if key.code == KeyCode::Char('i') {
            'i'
        } else {
            'a'
        });
        Action::Noop
    }

    fn arm_visual_find(&mut self, key: KeyEvent, find: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push(find);
        Action::BeginSequence
    }

    fn arm_visual_g(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push('g');
        Action::Noop
    }

    fn arm_visual_z(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending.push('z');
        Action::Noop
    }

    fn push_visual_digit(&mut self, key: KeyEvent, digit: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.push_digit(digit)
    }

    fn resolve_visual_pending(&mut self, key: KeyEvent, mode: Mode) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.clear_pending();
                Action::CancelVisual
            }
            KeyCode::Char(c) if !has_ctrl_or_alt(key) => self.visual_pending_key(c, mode),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => self.finish_visual_pending(key, mode),
            _ => self.cancel_sequence(),
        }
    }

    fn finish_visual_pending(&mut self, key: KeyEvent, mode: Mode) -> Action {
        if self.only_digits_pending() {
            self.clear_pending();
            return self.map_visual_fresh(key, mode);
        }
        self.cancel_sequence()
    }

    fn visual_pending_key(&mut self, pushed: char, mode: Mode) -> Action {
        if self.stripped_pending() == "g" {
            if pushed == 'g' {
                self.clear_pending();
                return Action::GotoFirstLine;
            }
            if pushed == 'v' {
                self.clear_pending();
                return Action::ReselectVisual;
            }
            if pushed == 'e' {
                self.clear_pending();
                return Action::MoveToPrevWordEnd;
            }
            if let Some(action) = display_motion_action(pushed) {
                self.clear_pending();
                return action;
            }
        }
        if self.stripped_pending() == "z" {
            if let Some(action) = place_action(pushed) {
                self.clear_pending();
                return action;
            }
        }
        if pushed == 'z' && self.only_digits_pending() {
            self.pending.push(pushed);
            return Action::Noop;
        }
        if pushed.is_ascii_digit() {
            return self.push_digit(pushed);
        }
        if let Some(action) = self.complete_visual_find(pushed) {
            return action;
        }
        if let Some(action) = self.complete_visual_object(pushed) {
            return action;
        }
        if let Some(action) = self.complete_visual_g(pushed) {
            return action;
        }
        if matches!(pushed, 'D' | 'Y') && self.only_digits_pending() {
            self.clear_pending();
            return if pushed == 'D' {
                Action::DeleteLines
            } else {
                Action::YankLines
            };
        }
        if matches!(pushed, 'C' | 'S') && self.only_digits_pending() {
            self.clear_pending();
            return Action::ChangeLines;
        }
        if matches!(pushed, '>' | '<') && self.only_digits_pending() {
            self.clear_pending();
            return if pushed == '>' {
                Action::ShiftRight
            } else {
                Action::ShiftLeft
            };
        }
        if pushed == '=' && self.only_digits_pending() {
            self.clear_pending();
            return Action::Reindent;
        }
        if matches!(pushed, '~' | 'u' | 'U') && self.only_digits_pending() {
            self.clear_pending();
            return self.case_action(pushed);
        }
        if matches!(pushed, ';' | ',') && self.only_digits_pending() {
            self.clear_pending();
            return self.repeat_motion(
                KeyEvent::new(KeyCode::Char(pushed), KeyModifiers::NONE),
                pushed == ',',
            );
        }
        if matches!(pushed, 'd' | 'y') && self.only_digits_pending() {
            self.clear_pending();
            return if pushed == 'd' {
                Action::DeleteSelection
            } else {
                Action::YankSelection
            };
        }
        if matches!(pushed, 'c' | 's') && self.only_digits_pending() {
            self.clear_pending();
            return Action::ChangeSelection;
        }
        if matches!(pushed, 'v' | 'V' | 'o' | 'O' | 'p' | 'c' | 's') {
            self.clear_pending();
            return self.map_visual_fresh(
                KeyEvent::new(KeyCode::Char(pushed), KeyModifiers::NONE),
                mode,
            );
        }
        if self.only_digits_pending() && is_visual_motion_key(pushed) {
            self.clear_pending();
            return self.map_visual_fresh(
                KeyEvent::new(KeyCode::Char(pushed), KeyModifiers::NONE),
                mode,
            );
        }
        self.cancel_sequence()
    }

    fn complete_visual_find(&mut self, target: char) -> Option<Action> {
        let (operator, find) = self.pending_find()?;
        if operator.is_some() {
            return None;
        }
        let forward = matches!(find, 'f' | 't');
        let till = matches!(find, 't' | 'T');
        self.last_find = Some(FindMemory {
            target,
            forward,
            till,
        });
        self.clear_pending();
        Some(Action::FindChar {
            target,
            forward,
            till,
        })
    }

    fn complete_visual_g(&mut self, pushed: char) -> Option<Action> {
        if self.stripped_pending() != "g" {
            return None;
        }
        let action = match pushed {
            '~' | 'u' | 'U' => self.case_action(pushed),
            _ => return None,
        };
        self.clear_pending();
        Some(action)
    }

    fn case_action(&self, pushed: char) -> Action {
        match pushed {
            '~' => Action::ToggleCase,
            'u' => Action::Lowercase,
            _ => Action::Uppercase,
        }
    }

    fn complete_visual_object(&mut self, pushed: char) -> Option<Action> {
        let stripped = self.stripped_pending();
        if stripped.len() != 1 || !matches!(stripped.as_str(), "i" | "a") {
            return None;
        }
        let inner = stripped == "i";
        let action = match pushed {
            'w' => {
                if inner {
                    Action::SelectInnerWord
                } else {
                    Action::SelectAroundWord
                }
            }
            '(' | ')' => Action::SelectObject {
                inner,
                object: Object::Paren,
            },
            '{' | '}' => Action::SelectObject {
                inner,
                object: Object::Brace,
            },
            '[' | ']' => Action::SelectObject {
                inner,
                object: Object::Bracket,
            },
            '<' | '>' => Action::SelectObject {
                inner,
                object: Object::Angle,
            },
            '\'' => Action::SelectObject {
                inner,
                object: Object::SingleQuote,
            },
            '"' => Action::SelectObject {
                inner,
                object: Object::DoubleQuote,
            },
            '`' => Action::SelectObject {
                inner,
                object: Object::Backtick,
            },
            'p' => Action::SelectObject {
                inner,
                object: Object::Paragraph,
            },
            _ => return None,
        };
        self.clear_pending();
        Some(action)
    }
}

impl Keymap for VimKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        let key = normalize_interrupt(key);
        match mode {
            Mode::Insert => self.map_insert(key),
            Mode::Normal => self.map_normal(key),
            Mode::Command => self.map_command(key),
            Mode::Visual | Mode::VisualLine => self.map_visual_mode(key, mode),
            Mode::Search => crate::keymap::map_search_mode(key),
        }
    }

    fn operator_exits_visual(&self) -> bool {
        true
    }

    fn pending_text(&self) -> Option<&str> {
        self.pending_key()
    }
}

impl Default for VimKeymap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn normal_h_moves_left() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('h')), Mode::Normal),
            Action::MoveLeft
        );
    }

    #[test]
    fn normal_ctrl_o_jumps_older() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('o')), Mode::Normal),
            Action::JumpOlder
        );
    }

    #[test]
    fn normal_ctrl_i_jumps_newer() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('i')), Mode::Normal),
            Action::JumpNewer
        );
    }

    #[test]
    fn normal_tab_jumps_newer() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Tab), Mode::Normal),
            Action::JumpNewer
        );
    }

    #[test]
    fn normal_i_enters_insert() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::EnterInsert
        );
    }

    #[test]
    fn insert_ctrl_t_shifts_right() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('t')), Mode::Insert),
            Action::InsertShiftRight
        );
    }

    #[test]
    fn insert_ctrl_d_shifts_left() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('d')), Mode::Insert),
            Action::InsertShiftLeft
        );
    }

    #[test]
    fn insert_tab_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Tab), Mode::Insert),
            Action::InsertTab
        );
    }

    #[test]
    fn insert_esc_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Insert),
            Action::ExitInsert
        );
    }

    #[test]
    fn normal_esc_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(map.map(plain(KeyCode::Esc), Mode::Normal), Action::Noop);
    }

    #[test]
    fn normal_enter_moves_to_next_line_start() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Enter), Mode::Normal),
            Action::MoveToNextLineStart
        );
    }

    #[test]
    fn normal_plus_minus_move_line_start() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('+')), Mode::Normal),
            Action::MoveToNextLineStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('-')), Mode::Normal),
            Action::MoveToPrevLineStart
        );
    }

    #[test]
    fn ctrl_s_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('s')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('s')), Mode::Insert),
            Action::Noop
        );
    }

    #[test]
    fn insert_char_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Insert),
            Action::InsertChar('q')
        );
    }

    #[test]
    fn normal_q_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Normal),
            Action::Noop
        );
    }

    #[test]
    fn ctrl_q_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Insert),
            Action::Noop
        );
    }

    #[test]
    fn ctrl_c_matches_esc_everywhere() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Insert),
            Action::ExitInsert
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Visual),
            Action::CancelVisual
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Command),
            Action::CancelCommand
        );
    }

    #[test]
    fn normal_x_deletes_forward() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::DeleteForward
        );
    }

    #[test]
    fn normal_dollar_moves_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::MoveToLineEnd
        );
    }

    #[test]
    fn dd_deletes_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::DeleteLine
        );
    }

    #[test]
    fn d_then_other_clears() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::MoveDown
        );
    }

    #[test]
    fn colon_enters_command() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char(':')), Mode::Normal),
            Action::EnterCommand
        );
    }

    #[test]
    fn command_chars_push() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Command),
            Action::PushCommandChar('w')
        );
    }

    #[test]
    fn command_enter_executes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Enter), Mode::Command),
            Action::ExecuteCommand
        );
    }

    #[test]
    fn command_esc_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Command),
            Action::CancelCommand
        );
    }

    #[test]
    fn normal_o_opens_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('o')), Mode::Normal),
            Action::OpenLineBelow
        );
    }

    #[test]
    fn normal_v_enters_visual() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::EnterVisual
        );
    }

    #[test]
    fn visual_esc_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Visual),
            Action::CancelVisual
        );
    }

    #[test]
    fn visual_motion_moves() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('l')), Mode::Visual),
            Action::MoveRight
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Visual),
            Action::MoveDown
        );
    }

    #[test]
    fn visual_v_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Visual),
            Action::CancelVisual
        );
    }

    #[test]
    fn normal_u_undoes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Normal),
            Action::Undo
        );
    }

    #[test]
    fn normal_ctrl_r_redoes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('r')), Mode::Normal),
            Action::Redo
        );
    }

    #[test]
    fn insert_u_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Insert),
            Action::InsertChar('u')
        );
    }

    #[test]
    fn insert_ctrl_r_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('r')), Mode::Insert),
            Action::Noop
        );
    }

    #[test]
    fn normal_a_appends() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Normal),
            Action::Append
        );
    }

    #[test]
    fn insert_a_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Insert),
            Action::InsertChar('a')
        );
    }

    #[test]
    fn capital_a_appends_at_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('A')), Mode::Normal),
            Action::AppendLineEnd
        );
    }

    #[test]
    fn capital_i_inserts_at_start() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('I')), Mode::Normal),
            Action::InsertLineStart
        );
    }

    #[test]
    fn normal_word_motions_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::MoveToNextWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('b')), Mode::Normal),
            Action::MoveToPrevWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::MoveToNextWordEnd
        );
    }

    #[test]
    fn visual_word_motions_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Visual),
            Action::MoveToNextWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Visual),
            Action::MoveToNextWordEnd
        );
    }

    #[test]
    fn visual_operators_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Visual),
            Action::DeleteSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Visual),
            Action::YankSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Visual),
            Action::ChangeSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('s')), Mode::Visual),
            Action::ChangeSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::VisualLine),
            Action::ChangeSelection
        );
    }

    #[test]
    fn visual_count_c_changes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Visual),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Visual),
            Action::ChangeSelection
        );
    }

    #[test]
    fn normal_big_v_enters_linewise() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('V')), Mode::Normal),
            Action::EnterVisualLine
        );
    }

    #[test]
    fn visual_line_operators_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('D')), Mode::Visual),
            Action::DeleteLines
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('Y')), Mode::Visual),
            Action::YankLines
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('C')), Mode::Visual),
            Action::ChangeLines
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('S')), Mode::Visual),
            Action::ChangeLines
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('D')), Mode::VisualLine),
            Action::DeleteLines
        );
    }

    #[test]
    fn visual_case_operators_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::Visual),
            Action::ToggleCase
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Visual),
            Action::Lowercase
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('U')), Mode::Visual),
            Action::Uppercase
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::VisualLine),
            Action::ToggleCase
        );
    }

    #[test]
    fn visual_g_case_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Visual),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::Visual),
            Action::ToggleCase
        );
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Visual),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Visual),
            Action::Lowercase
        );
    }

    #[test]
    fn visual_g_invalid_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Visual),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Visual),
            Action::CancelPending
        );
    }

    #[test]
    fn visual_count_capital_d_deletes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Visual),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('D')), Mode::Visual),
            Action::DeleteLines
        );
    }

    #[test]
    fn normal_g_arms_case_operator() {
        for operator in ['~', 'u', 'U'] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char('g')), Mode::Normal),
                Action::Noop
            );
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                Action::BeginSequence
            );
        }
    }

    #[test]
    fn normal_g_digit_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::CancelPending
        );
    }

    #[test]
    fn normal_case_word_motions_map() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::WordForward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('u')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('b')), Mode::Normal),
            Action::LowercaseAt {
                motion: CaseMotion::WordBackward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('U')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::UppercaseAt {
                motion: CaseMotion::WordEnd,
            }
        );
    }

    #[test]
    fn normal_case_line_motions_map() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::LineEnd,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::LineStart,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('U')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::UppercaseAt {
                motion: CaseMotion::Lines { upward: false },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('u')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('k')), Mode::Normal),
            Action::LowercaseAt {
                motion: CaseMotion::Lines { upward: true },
            }
        );
    }

    #[test]
    fn normal_case_double_operator_is_linewise() {
        for (operator, action) in [
            (
                '~',
                Action::ToggleCaseAt {
                    motion: CaseMotion::OperatorLines,
                },
            ),
            (
                'u',
                Action::LowercaseAt {
                    motion: CaseMotion::OperatorLines,
                },
            ),
            (
                'U',
                Action::UppercaseAt {
                    motion: CaseMotion::OperatorLines,
                },
            ),
        ] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            map.map(plain(KeyCode::Char(operator)), Mode::Normal);
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                action
            );
        }
    }

    #[test]
    fn normal_case_mixed_operators_cancel() {
        for second in ['u', 'U'] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            map.map(plain(KeyCode::Char('~')), Mode::Normal);
            assert_eq!(
                map.map(plain(KeyCode::Char(second)), Mode::Normal),
                Action::CancelPending
            );
        }
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('u')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::Normal),
            Action::CancelPending
        );
    }

    #[test]
    fn normal_case_find_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('f')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::Find {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            }
        );
    }

    #[test]
    fn normal_case_object_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('U')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('(')), Mode::Normal),
            Action::UppercaseAt {
                motion: CaseMotion::Object {
                    inner: true,
                    object: Object::Paren,
                },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        map.map(plain(KeyCode::Char('i')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::CancelPending
        );
    }

    #[test]
    fn normal_case_repeat_find_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::CancelPending
        );
        let mut map = VimKeymap::new();
        for key in ['f', 'x'] {
            map.map(plain(KeyCode::Char(key)), Mode::Normal);
        }
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::RepeatFind {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            }
        );
    }

    #[test]
    fn normal_case_counts_flow() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::WordForward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::WordForward,
            }
        );
    }

    #[test]
    fn normal_ge_moves_to_prev_word_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::MoveToPrevWordEnd
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_g_arms_without_digits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(map.pending_text(), Some("2g"));
    }

    #[test]
    fn normal_ge_sequences_resolve() {
        assert_eq!(sequence_action("dge"), Some(Action::DeleteWordEndBackward));
        assert_eq!(sequence_action("cge"), Some(Action::ChangeWordEndBackward));
        assert_eq!(sequence_action("yge"), Some(Action::YankWordEndBackward));
    }

    #[test]
    fn normal_ge_count_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(map.pending_text(), Some("2g"));
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::MoveToPrevWordEnd
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_case_ge_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::WordEndBackward,
            }
        );
    }

    #[test]
    fn visual_ge_extends() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Visual);
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Visual),
            Action::MoveToPrevWordEnd
        );
    }

    #[test]
    fn normal_tilde_toggles_char() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('~')), Mode::Normal),
            Action::ToggleChar
        );
    }

    #[test]
    fn normal_g_display_motions_map() {
        for (key, action) in [
            ('j', Action::MoveDown),
            ('k', Action::MoveUp),
            ('0', Action::MoveToLineStart),
            ('$', Action::MoveToLineEnd),
        ] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Normal), action);
        }
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('I')), Mode::Normal),
            Action::InsertAtColumnStart
        );
    }

    #[test]
    fn visual_g_display_motions_map() {
        for (key, action) in [
            ('j', Action::MoveDown),
            ('k', Action::MoveUp),
            ('0', Action::MoveToLineStart),
            ('$', Action::MoveToLineEnd),
        ] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Visual);
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Visual), action);
        }
    }

    #[test]
    fn normal_goto_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('G')), Mode::Normal),
            Action::GotoLastLine
        );
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::GotoFirstLine
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_goto_operators_map() {
        for (first, second, action) in [
            ('d', 'G', Action::DeleteToLastLine),
            ('c', 'G', Action::ChangeToLastLine),
            ('y', 'G', Action::YankToLastLine),
        ] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char(first)), Mode::Normal);
            assert_eq!(map.map(plain(KeyCode::Char(second)), Mode::Normal), action);
        }
        for (first, action) in [
            ('d', Action::DeleteToFirstLine),
            ('c', Action::ChangeToFirstLine),
            ('y', Action::YankToFirstLine),
        ] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char(first)), Mode::Normal);
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            assert_eq!(map.map(plain(KeyCode::Char('g')), Mode::Normal), action);
        }
    }

    #[test]
    fn viewport_operators_map() {
        for (keys, action) in [
            ("dH", Action::DeleteToViewportTop),
            ("dM", Action::DeleteToViewportMiddle),
            ("dL", Action::DeleteToViewportBottom),
            ("cH", Action::ChangeToViewportTop),
            ("cM", Action::ChangeToViewportMiddle),
            ("cL", Action::ChangeToViewportBottom),
            ("yH", Action::YankToViewportTop),
            ("yM", Action::YankToViewportMiddle),
            ("yL", Action::YankToViewportBottom),
        ] {
            let mut map = VimKeymap::new();
            let mut resolved = Action::Noop;
            for key in keys.chars() {
                resolved = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            }
            assert_eq!(resolved, action);
            assert_eq!(map.pending_text(), None);
        }
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('H')), Mode::Normal),
            Action::DeleteToViewportTop
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn match_motion_and_operators_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('%')), Mode::Normal),
            Action::MoveToMatch
        );
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('%')), Mode::Visual),
            Action::MoveToMatch
        );
        for (keys, action) in [
            ("d%", Action::DeleteToMatch),
            ("c%", Action::ChangeToMatch),
            ("y%", Action::YankToMatch),
        ] {
            let mut map = VimKeymap::new();
            let mut resolved = Action::Noop;
            for key in keys.chars() {
                resolved = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            }
            assert_eq!(resolved, action);
            assert_eq!(map.pending_text(), None);
        }
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('5')), Mode::Normal),
            Action::PushCount(5)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::PushCount(0)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('%')), Mode::Normal),
            Action::DeleteToMatch
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_case_goto_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('G')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::GotoLine { last: true },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        map.map(plain(KeyCode::Char('~')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::ToggleCaseAt {
                motion: CaseMotion::GotoLine { last: false },
            }
        );
    }

    #[test]
    fn visual_goto_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('G')), Mode::Visual),
            Action::GotoLastLine
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('G')), Mode::VisualLine),
            Action::GotoLastLine
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Visual);
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Visual),
            Action::GotoFirstLine
        );
    }

    #[test]
    fn normal_g_unimplemented_second_keys_cancel() {
        let implemented = ['~', 'u', 'U', 'g', 'v', 'i', 'e', 'I', 'j', 'k', '0', '$'];
        for code in 0..128u8 {
            let pushed = code as char;
            if !pushed.is_ascii_graphic() || pushed == 'g' {
                continue;
            }
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            let action = map.map(plain(KeyCode::Char(pushed)), Mode::Normal);
            if implemented.contains(&pushed) {
                continue;
            }
            assert_eq!(action, Action::CancelPending, "g{pushed}");
            assert_eq!(
                map.map(plain(KeyCode::Char('l')), Mode::Normal),
                Action::MoveRight,
                "clean after g{pushed}"
            );
        }
    }

    #[test]
    fn normal_case_operator_invalid_third_cancels() {
        for pushed in ['x', 'q', 'd', ';'] {
            let mut map = VimKeymap::new();
            map.map(plain(KeyCode::Char('g')), Mode::Normal);
            map.map(plain(KeyCode::Char('~')), Mode::Normal);
            assert_eq!(
                map.map(plain(KeyCode::Char(pushed)), Mode::Normal),
                Action::CancelPending,
                "g~{pushed}"
            );
            assert_eq!(
                map.map(plain(KeyCode::Char('l')), Mode::Normal),
                Action::MoveRight,
                "clean after g~{pushed}"
            );
        }
    }

    #[test]
    fn normal_gi_inserts_at_last() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::InsertAtLast
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_gv_reselects() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::ReselectVisual
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn normal_gv_count_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::ReselectVisual
        );
    }

    #[test]
    fn visual_gv_reselects() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Visual);
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Visual),
            Action::ReselectVisual
        );
    }

    #[test]
    fn visual_gi_cancels() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Visual);
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Visual),
            Action::CancelPending
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('l')), Mode::Visual),
            Action::MoveRight
        );
    }

    #[test]
    fn visual_g_junk_cancels() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('g')), Mode::Visual);
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Visual),
            Action::CancelPending
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('l')), Mode::Visual),
            Action::MoveRight
        );
    }

    #[test]
    fn visual_line_toggle_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('V')), Mode::Visual),
            Action::EnterVisualLine
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('V')), Mode::VisualLine),
            Action::CancelVisual
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::VisualLine),
            Action::EnterVisual
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('o')), Mode::VisualLine),
            Action::SwapSelectionEnds
        );
    }

    #[test]
    fn normal_p_pastes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('p')), Mode::Normal),
            Action::PasteAfter
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('P')), Mode::Normal),
            Action::PasteBefore
        );
    }

    #[test]
    fn pending_d_then_v_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::EnterVisual
        );
    }

    #[test]
    fn double_c_changes_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::ChangeLine
        );
    }

    #[test]
    fn c_w_changes_word() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ChangeWord
        );
    }

    #[test]
    fn d_w_deletes_word() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::DeleteWord
        );
    }

    #[test]
    fn d_b_deletes_back() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('b')), Mode::Normal),
            Action::DeleteWordBackward
        );
    }

    #[test]
    fn c_e_changes_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::ChangeWordEnd
        );
    }

    #[test]
    fn d_e_deletes_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::DeleteWordEnd
        );
    }

    #[test]
    fn pending_key_tracks_sequence() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('d')), Mode::Normal);
        assert_eq!(map.pending_text(), Some("d"));
        map.map(plain(KeyCode::Char('d')), Mode::Normal);
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn pending_key_clears_on_other_key() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('c')), Mode::Normal);
        assert_eq!(map.pending_text(), Some("c"));
        map.map(plain(KeyCode::Char('j')), Mode::Normal);
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn unknown_second_key_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn diw_daw_resolve() {
        for (keys, action) in [
            ("diw", Action::DeleteInnerWord),
            ("daw", Action::DeleteAroundWord),
            ("ciw", Action::ChangeInnerWord),
            ("caw", Action::ChangeAroundWord),
            ("yiw", Action::YankInnerWord),
            ("yaw", Action::YankAroundWord),
        ] {
            let mut map = VimKeymap::new();
            let mut resolved = Action::Noop;
            for key in keys.chars() {
                resolved = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            }
            assert_eq!(resolved, action);
        }
    }

    #[test]
    fn object_sequences_resolve() {
        for (keys, action) in [
            (
                "di(",
                Action::DeleteObject {
                    inner: true,
                    object: Object::Paren,
                },
            ),
            (
                "da(",
                Action::DeleteObject {
                    inner: false,
                    object: Object::Paren,
                },
            ),
            (
                "di)",
                Action::DeleteObject {
                    inner: true,
                    object: Object::Paren,
                },
            ),
            (
                "ci{",
                Action::ChangeObject {
                    inner: true,
                    object: Object::Brace,
                },
            ),
            (
                "ya[",
                Action::YankObject {
                    inner: false,
                    object: Object::Bracket,
                },
            ),
            (
                "di<",
                Action::DeleteObject {
                    inner: true,
                    object: Object::Angle,
                },
            ),
            (
                "di'",
                Action::DeleteObject {
                    inner: true,
                    object: Object::SingleQuote,
                },
            ),
            (
                "da\"",
                Action::DeleteObject {
                    inner: false,
                    object: Object::DoubleQuote,
                },
            ),
            (
                "yi`",
                Action::YankObject {
                    inner: true,
                    object: Object::Backtick,
                },
            ),
            (
                "dip",
                Action::DeleteObject {
                    inner: true,
                    object: Object::Paragraph,
                },
            ),
            (
                "cap",
                Action::ChangeObject {
                    inner: false,
                    object: Object::Paragraph,
                },
            ),
        ] {
            let mut map = VimKeymap::new();
            let mut resolved = Action::Noop;
            for key in keys.chars() {
                resolved = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            }
            assert_eq!(resolved, action);
        }
    }

    #[test]
    fn unknown_object_cancels() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('d')), Mode::Normal);
        map.map(plain(KeyCode::Char('i')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn object_with_count_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(map.pending_text(), Some("2di"));
        assert_eq!(
            map.map(plain(KeyCode::Char('(')), Mode::Normal),
            Action::DeleteObject {
                inner: true,
                object: Object::Paren,
            }
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn count_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor =
            kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("foo bar qux baz"));
        for key in ['2', 'd', '2', 'w'] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 2), vec![String::new()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn object_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor =
            kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("foo (bar) baz"));
        for key in ['d', 'i', '('] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 2), vec!["foo () baz".to_string()]);
        assert_eq!((editor.row(), editor.col()), (0, 5));
    }

    #[test]
    fn doubled_count_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor = kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("a\nb\nc"));
        for key in ['2', 'd', 'd'] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 3), vec!["c".to_string()]);
    }

    #[test]
    fn find_arms_track_pending() {
        for (find, shown) in [('f', "f"), ('F', "F"), ('t', "t"), ('T', "T")] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(find)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.pending_text(), Some(shown));
        }
    }

    #[test]
    fn find_completes_with_target() {
        let cases = [
            (
                'f',
                Action::FindChar {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            ),
            (
                'F',
                Action::FindChar {
                    target: 'x',
                    forward: false,
                    till: false,
                },
            ),
            (
                't',
                Action::FindChar {
                    target: 'x',
                    forward: true,
                    till: true,
                },
            ),
            (
                'T',
                Action::FindChar {
                    target: 'x',
                    forward: false,
                    till: true,
                },
            ),
        ];
        for (find, action) in cases {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(find)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.map(plain(KeyCode::Char('x')), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn find_digit_is_target() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::FindChar {
                target: '2',
                forward: true,
                till: false,
            }
        );
    }

    #[test]
    fn find_with_operator() {
        let cases = [
            (
                'd',
                Action::DeleteFind {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            ),
            (
                'c',
                Action::ChangeFind {
                    target: 'x',
                    forward: true,
                    till: true,
                },
            ),
            (
                'y',
                Action::YankFind {
                    target: 'x',
                    forward: false,
                    till: false,
                },
            ),
        ];
        let finds = ['f', 't', 'F'];
        for ((operator, action), find) in cases.into_iter().zip(finds) {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(
                map.map(plain(KeyCode::Char(find)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.map(plain(KeyCode::Char('x')), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn find_with_counts_on_both_sides() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('f')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::DeleteFind {
                target: 'x',
                forward: true,
                till: false,
            }
        );
    }

    #[test]
    fn repeat_without_find_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char(',')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn repeat_uses_last_find() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        map.map(plain(KeyCode::Char('x')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::RepeatFind {
                target: 'x',
                forward: true,
                till: false,
            }
        );
        assert_eq!(
            map.map(plain(KeyCode::Char(',')), Mode::Normal),
            Action::RepeatFind {
                target: 'x',
                forward: false,
                till: false,
            }
        );
    }

    #[test]
    fn repeat_failed_search_sticks() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        map.map(plain(KeyCode::Char('b')), Mode::Normal);
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        map.map(plain(KeyCode::Char('z')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::RepeatFind {
                target: 'z',
                forward: true,
                till: false,
            }
        );
    }

    #[test]
    fn repeat_with_operator() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        map.map(plain(KeyCode::Char('b')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::DeleteRepeatFind {
                target: 'b',
                forward: true,
                till: false,
            }
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn repeat_without_find_cancels_pending() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('d')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn esc_during_find_cancels() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('f')), Mode::Normal);
        assert_eq!(map.pending_text(), Some("f"));
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn find_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor = kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("axbxc"));
        for key in ['d', 'f', 'x'] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 2), vec!["bxc".to_string()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn shift_sequences_resolve() {
        assert_eq!(sequence_action(">>"), Some(Action::ShiftRight));
        assert_eq!(sequence_action("<<"), Some(Action::ShiftLeft));
        assert_eq!(sequence_action("=="), Some(Action::Reindent));
        assert!(is_sequence_prefix(">"));
        assert!(is_sequence_prefix("<"));
        assert!(is_sequence_prefix("="));
    }

    #[test]
    fn shift_count_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Normal),
            Action::ShiftRight
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn visual_shift_maps() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Visual),
            Action::ShiftRight
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('<')), Mode::Visual),
            Action::ShiftLeft
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('=')), Mode::Visual),
            Action::Reindent
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::VisualLine),
            Action::ShiftRight
        );
    }

    #[test]
    fn visual_shift_count_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Visual),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Visual),
            Action::ShiftRight
        );
    }

    #[test]
    fn shift_word_motions_map() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::WordForward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('<')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('b')), Mode::Normal),
            Action::ShiftLeftAt {
                motion: ShiftMotion::WordBackward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('=')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::ReindentAt {
                motion: ShiftMotion::WordEnd,
            }
        );
    }

    #[test]
    fn shift_line_motions_map() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::LineEnd,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::LineStart,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::Lines { upward: false },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('<')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('k')), Mode::Normal),
            Action::ShiftLeftAt {
                motion: ShiftMotion::Lines { upward: true },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('G')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::GotoLine { last: true },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('=')), Mode::Normal);
        map.map(plain(KeyCode::Char('g')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('g')), Mode::Normal),
            Action::ReindentAt {
                motion: ShiftMotion::GotoLine { last: false },
            }
        );
    }

    #[test]
    fn shift_find_object_map() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('f')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::Find {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('<')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('(')), Mode::Normal),
            Action::ShiftLeftAt {
                motion: ShiftMotion::Object {
                    inner: true,
                    object: Object::Paren,
                },
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('=')), Mode::Normal);
        map.map(plain(KeyCode::Char('i')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::CancelPending
        );
    }

    #[test]
    fn shift_repeat_find_maps() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::CancelPending
        );
        let mut map = VimKeymap::new();
        for key in ['f', 'x'] {
            map.map(plain(KeyCode::Char(key)), Mode::Normal);
        }
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::RepeatFind {
                    target: 'x',
                    forward: true,
                    till: false,
                },
            }
        );
    }

    #[test]
    fn digit_operator_keeps_pending_display() {
        for operator in ['d', '>', '<', '=', 'f', 'g'] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char('2')), Mode::Normal),
                Action::PushCount(2)
            );
            map.map(plain(KeyCode::Char(operator)), Mode::Normal);
            assert_eq!(
                map.pending_text(),
                Some(match operator {
                    'd' => "2d",
                    '>' => "2>",
                    '<' => "2<",
                    '=' => "2=",
                    'f' => "2f",
                    _ => "2g",
                })
            );
        }
    }

    #[test]
    fn digit_shift_completes() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('2')), Mode::Normal);
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::WordForward,
            }
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn shift_counts_flow() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::WordForward,
            }
        );
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('>')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::ShiftRightAt {
                motion: ShiftMotion::Lines { upward: false },
            }
        );
    }

    #[test]
    fn shift_double_operator_still_resolves() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('>')), Mode::Normal),
            Action::ShiftRight
        );
    }

    #[test]
    fn dot_repeats_last_change() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('.')), Mode::Normal),
            Action::RepeatLastChange
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn dot_with_pending_cancels() {
        let mut map = VimKeymap::new();
        map.map(plain(KeyCode::Char('d')), Mode::Normal);
        assert_eq!(
            map.map(plain(KeyCode::Char('.')), Mode::Normal),
            Action::CancelPending
        );
    }

    #[test]
    fn dot_in_visual_stays() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('.')), Mode::Visual),
            Action::Noop
        );
    }

    #[test]
    fn dollar_sequences_flow() {
        for (operator, action) in [
            ('d', Action::DeleteToEnd),
            ('c', Action::ChangeToEnd),
            ('y', Action::YankToEnd),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.map(plain(KeyCode::Char('$')), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn dollar_count_after_operator_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::DeleteToEnd
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn dollar_count_before_operator_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::DeleteToEnd
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn dollar_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor =
            kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("abcdef\nghijkl"));
        for key in ['d', '2', '$'] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 2), vec![String::new()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn repeat_flows_into_editor() {
        let mut map = VimKeymap::new();
        let mut editor = kawi_editor::Editor::new(kawi_buffer::Buffer::from_string("axbxc"));
        for key in ['f', 'b', '0', 'd', ';'] {
            let action = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            editor.apply(action);
        }
        assert_eq!(editor.visible_lines(0, 2), vec!["xc".to_string()]);
    }

    #[test]
    fn sequence_table_resolves() {
        assert_eq!(sequence_action("dd"), Some(Action::DeleteLine));
        assert_eq!(sequence_action("dw"), Some(Action::DeleteWord));
        assert_eq!(sequence_action("db"), Some(Action::DeleteWordBackward));
        assert_eq!(sequence_action("de"), Some(Action::DeleteWordEnd));
        assert_eq!(sequence_action("d0"), Some(Action::DeleteToStart));
        assert_eq!(sequence_action("d$"), Some(Action::DeleteToEnd));
        assert_eq!(sequence_action("c0"), Some(Action::ChangeToStart));
        assert_eq!(sequence_action("c$"), Some(Action::ChangeToEnd));
        assert_eq!(sequence_action("y0"), Some(Action::YankToStart));
        assert_eq!(sequence_action("y$"), Some(Action::YankToEnd));
        assert_eq!(sequence_action("dx"), None);
    }

    #[test]
    fn sequence_prefix_detects_extendable() {
        assert!(is_sequence_prefix("d"));
        assert!(!is_sequence_prefix("dd"));
        assert!(!is_sequence_prefix("dx"));
    }

    #[test]
    fn double_y_yanks_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Normal),
            Action::YankLine
        );
    }

    #[test]
    fn digits_push_count() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(map.pending_text(), Some("2"));
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::PushCount(0)
        );
        assert_eq!(map.pending_text(), Some("20"));
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::MoveToNextWordStart
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn fresh_zero_moves_to_line_start() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::MoveToLineStart
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn count_before_operator_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(map.pending_text(), Some("2d"));
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::DeleteWord
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn count_after_operator_flows() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(map.pending_text(), Some("d2"));
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::DeleteWord
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn doubled_operator_takes_count() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::DeleteLine
        );
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::DeleteLine
        );
    }

    #[test]
    fn operator_zero_sequences_resolve() {
        for (operator, action) in [
            ('d', Action::DeleteToStart),
            ('c', Action::ChangeToStart),
            ('y', Action::YankToStart),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.map(plain(KeyCode::Char('0')), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn caret_motion_and_sequences_resolve() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('^')), Mode::Normal),
            Action::MoveToFirstNonBlank
        );
        for (operator, action) in [
            ('d', Action::DeleteToFirstNonBlank),
            ('c', Action::ChangeToFirstNonBlank),
            ('y', Action::YankToFirstNonBlank),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char(operator)), Mode::Normal),
                Action::BeginSequence
            );
            assert_eq!(map.map(plain(KeyCode::Char('^')), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
        }
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('^')), Mode::Normal),
            Action::DeleteToFirstNonBlank
        );
        assert_eq!(map.pending_text(), None);
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('^')), Mode::Visual),
            Action::MoveToFirstNonBlank
        );
    }

    #[test]
    fn statement_motions_and_operators_map() {
        for (key, action) in [
            ('(', Action::MoveToPrevSentence),
            (')', Action::MoveToNextSentence),
            ('{', Action::MoveToPrevParagraph),
            ('}', Action::MoveToNextParagraph),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Normal), action);
            let mut map = VimKeymap::new();
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Visual), action);
        }
        for (keys, action) in [
            ("d(", Action::DeleteToPrevSentence),
            ("d)", Action::DeleteToNextSentence),
            ("d{", Action::DeleteToPrevParagraph),
            ("d}", Action::DeleteToNextParagraph),
            ("c(", Action::ChangeToPrevSentence),
            ("c)", Action::ChangeToNextSentence),
            ("c{", Action::ChangeToPrevParagraph),
            ("c}", Action::ChangeToNextParagraph),
            ("y(", Action::YankToPrevSentence),
            ("y)", Action::YankToNextSentence),
            ("y{", Action::YankToPrevParagraph),
            ("y}", Action::YankToNextParagraph),
        ] {
            let mut map = VimKeymap::new();
            let mut resolved = Action::Noop;
            for key in keys.chars() {
                resolved = map.map(plain(KeyCode::Char(key)), Mode::Normal);
            }
            assert_eq!(resolved, action);
            assert_eq!(map.pending_text(), None);
        }
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('}')), Mode::Normal),
            Action::DeleteToNextParagraph
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn ctrl_scroll_keys_resolve() {
        for (key, action) in [
            ('u', Action::ScrollHalfUp),
            ('d', Action::ScrollHalfDown),
            ('f', Action::ScrollPageDown),
            ('b', Action::ScrollPageUp),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(map.map(ctrl(KeyCode::Char(key)), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
            let mut map = VimKeymap::new();
            assert_eq!(map.map(ctrl(KeyCode::Char(key)), Mode::Visual), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn ctrl_scroll_keeps_count_and_clears_pending() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('d')), Mode::Normal),
            Action::ScrollHalfDown
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn viewport_motions_resolve() {
        for (key, action) in [
            ('H', Action::MoveToViewportTop),
            ('M', Action::MoveToViewportMiddle),
            ('L', Action::MoveToViewportBottom),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Normal), action);
            let mut map = VimKeymap::new();
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Visual), action);
        }
    }

    #[test]
    fn place_sequences_resolve() {
        for (key, action) in [
            ('z', Action::PlaceCenter),
            ('t', Action::PlaceTop),
            ('b', Action::PlaceBottom),
        ] {
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char('z')), Mode::Normal),
                Action::Noop
            );
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Normal), action);
            assert_eq!(map.pending_text(), None);
            let mut map = VimKeymap::new();
            assert_eq!(
                map.map(plain(KeyCode::Char('z')), Mode::Visual),
                Action::Noop
            );
            assert_eq!(map.map(plain(KeyCode::Char(key)), Mode::Visual), action);
            assert_eq!(map.pending_text(), None);
        }
    }

    #[test]
    fn place_sequence_with_count_resolves() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('3')), Mode::Normal),
            Action::PushCount(3)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('0')), Mode::Normal),
            Action::PushCount(0)
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('z')), Mode::Normal),
            Action::BeginSequence
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('z')), Mode::Normal),
            Action::PlaceCenter
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn place_sequence_junk_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('z')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn esc_with_pending_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('2')), Mode::Normal),
            Action::PushCount(2)
        );
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Normal),
            Action::CancelPending
        );
        assert_eq!(map.pending_text(), None);
    }

    #[test]
    fn capital_y_yanks_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('Y')), Mode::Normal),
            Action::YankToEnd
        );
    }

    #[test]
    fn capital_d_deletes_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('D')), Mode::Normal),
            Action::DeleteToEnd
        );
    }

    #[test]
    fn capital_c_changes_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('C')), Mode::Normal),
            Action::ChangeToEnd
        );
    }

    #[test]
    fn search_keys_map_in_normal() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('/')), Mode::Normal),
            Action::EnterSearch
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('n')), Mode::Normal),
            Action::NextMatch
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('N')), Mode::Normal),
            Action::PrevMatch
        );
    }

    #[test]
    fn search_prompt_maps_edits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Search),
            Action::PushSearchChar('a')
        );
        assert_eq!(
            map.map(plain(KeyCode::Enter), Mode::Search),
            Action::ConfirmSearch
        );
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Search),
            Action::CancelSearch
        );
        assert_eq!(
            map.map(plain(KeyCode::Backspace), Mode::Search),
            Action::DeleteSearchChar
        );
    }
}
