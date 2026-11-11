use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_lsp::SignatureTrigger;

use super::super::delayed::Delayed;
use super::super::App;
use super::DEFAULT_OPEN;

const SIGNATURE_RETRIGGER_DELAY: Duration = Duration::from_millis(120);
const SIGNATURE_CONTEXT_BOUND: usize = 65_536;

#[derive(Clone, PartialEq, Eq)]
pub(in crate::app) struct SignatureSchedule {
    character: Option<String>,
    open: char,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct SignatureSite {
    buffer: usize,
    row: usize,
    col: usize,
}

pub(in crate::app) type SignatureAuto = Delayed<SignatureSchedule, SignatureSite>;

impl App {
    pub(in crate::app) fn signature_wait_timeout(&self) -> Option<Duration> {
        self.signature_auto.wait_timeout()
    }

    pub(in crate::app) fn signature_tick(&mut self) -> bool {
        let now = Instant::now();
        if !self.signature_auto.is_due(now) {
            return false;
        }
        let site = self.signature_site();
        let Some(schedule) = self.signature_auto.take_due(now, site) else {
            return false;
        };
        if !self.signature_request_allowed() {
            self.hide_signature();
            return true;
        }
        if !self.inside_signature_context(schedule.open) {
            self.hide_signature();
            return true;
        }
        let trigger = match (self.signature.as_ref(), schedule.character.as_deref()) {
            (Some(session), Some(character)) => {
                SignatureTrigger::retrigger_character(character, session.event())
            }
            (Some(session), None) => SignatureTrigger::content_change(session.event()),
            (None, Some(character)) => SignatureTrigger::character(character),
            (None, None) => return false,
        };
        self.signature_open = schedule.open;
        if self.lsp.signature_help(&self.editor, trigger).is_err() {
            self.hide_signature();
            return true;
        }
        false
    }

    pub(in crate::app) fn after_signature_key(&mut self, key: KeyEvent) {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        let character = match key.code {
            KeyCode::Char(character) => Some(character),
            _ => None,
        };
        if self.signature.is_none() && character.is_none() {
            return;
        }
        if !self.signature_request_allowed() {
            self.hide_signature();
            return;
        }
        let capabilities = self.lsp.completion_capabilities();
        if !capabilities.signature_help {
            self.hide_signature();
            return;
        }
        if let Some(character) = character {
            if capabilities
                .signature_triggers
                .iter()
                .any(|trigger| trigger == &character.to_string())
            {
                self.schedule_trigger(character);
                return;
            }
        }
        let Some(session) = self.signature.as_ref() else {
            return;
        };
        let open = session.open();
        if !self.inside_signature_context(open) {
            self.hide_signature();
            return;
        }
        let retrigger = character.filter(|character| {
            capabilities
                .signature_retriggers
                .iter()
                .any(|trigger| trigger == &character.to_string())
        });
        self.schedule_refresh(retrigger.map(|character| character.to_string()), open);
    }

    pub(in crate::app) fn hide_signature(&mut self) {
        self.signature_auto.cancel();
        self.signature = None;
        self.lsp.cancel_signature();
    }

    fn schedule_trigger(&mut self, character: char) {
        let open = self.signature_open_character(character);
        if !self.inside_signature_context(open) {
            self.hide_signature();
            return;
        }
        let site = self.signature_site();
        self.signature_auto.schedule(
            SignatureSchedule {
                character: Some(character.to_string()),
                open,
            },
            Duration::ZERO,
            site,
        );
    }

    fn schedule_refresh(&mut self, character: Option<String>, open: char) {
        let delay = if character.is_some() {
            Duration::ZERO
        } else {
            SIGNATURE_RETRIGGER_DELAY
        };
        let site = self.signature_site();
        self.signature_auto
            .schedule(SignatureSchedule { character, open }, delay, site);
    }

    fn signature_site(&self) -> SignatureSite {
        SignatureSite {
            buffer: self.editor.buffer_number(),
            row: self.editor.row(),
            col: self.editor.col(),
        }
    }

    fn inside_signature_context(&self, open: char) -> bool {
        self.editor
            .inside_brackets(&[open], SIGNATURE_CONTEXT_BOUND)
    }

    fn signature_open_character(&self, character: char) -> char {
        self.editor
            .language_editing()
            .match_pairs
            .iter()
            .find(|pair| pair.open == character)
            .map(|pair| pair.open)
            .unwrap_or(DEFAULT_OPEN)
    }

    fn signature_request_allowed(&self) -> bool {
        self.completion_options.signature
            && self.editor.mode().is_insert()
            && self.completion.is_none()
            && self.picker.is_none()
            && self.replace.is_none()
            && self.pager.is_none()
    }
}
