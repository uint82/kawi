use crossterm::event::KeyEvent;

use super::is_pager_dismiss;

pub(super) trait Scrollable {
    fn scroll_by(&mut self, delta: isize, limit: usize);
}

pub(super) struct ScrollKey<'a, T> {
    pub(super) session: &'a mut Option<T>,
    pub(super) geometry: Option<(usize, usize)>,
    pub(super) key: KeyEvent,
    pub(super) up: KeyEvent,
    pub(super) down: KeyEvent,
}

pub(super) fn handle_scroll_key<T: Scrollable>(context: ScrollKey<'_, T>) -> Option<bool> {
    let ScrollKey {
        session,
        geometry,
        key,
        up,
        down,
    } = context;
    if session.is_none() {
        return None;
    }
    if key != up && key != down {
        *session = None;
        return is_pager_dismiss(key).then_some(false);
    }
    let Some((limit, page)) = geometry else {
        *session = None;
        return None;
    };
    let page = isize::try_from(page).unwrap_or(isize::MAX);
    let direction: isize = if key == up { -1 } else { 1 };
    let active = session.as_mut()?;
    active.scroll_by(direction.saturating_mul(page), limit);
    Some(false)
}
