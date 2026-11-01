use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::TextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextInputOutcome {
    Consumed,
    Submit,
    Cancel,
}

pub(crate) fn handle_text_input(
    input: &mut TextInput,
    key: KeyEvent,
    max_chars: usize,
) -> TextInputOutcome {
    if is_cancel(key) {
        return TextInputOutcome::Cancel;
    }
    if key.code == KeyCode::Enter {
        return TextInputOutcome::Submit;
    }
    edit_input(input, key, max_chars);
    TextInputOutcome::Consumed
}

fn edit_input(input: &mut TextInput, key: KeyEvent, max_chars: usize) {
    match key.code {
        KeyCode::Left => input.move_left(),
        KeyCode::Right => input.move_right(),
        KeyCode::Home => input.move_home(),
        KeyCode::End => input.move_end(),
        KeyCode::Backspace => input.backspace(),
        KeyCode::Delete => input.delete(),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => input.clear(),
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            input.delete_word();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            insert_text(input, &character.to_string(), max_chars);
        }
        _ => {}
    }
}

pub(crate) fn insert_text(input: &mut TextInput, text: &str, max_chars: usize) {
    for character in text.chars().filter(|character| !character.is_control()) {
        if input.text().chars().count() >= max_chars {
            break;
        }
        input.insert_char(character);
    }
}

pub(crate) fn is_cancel(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
}
