use super::*;

impl App {
    pub(crate) fn handle_debug_command(
        &mut self,
        command: kawi_editor::DebugCommand,
    ) -> anyhow::Result<bool> {
        use kawi_editor::DebugCommand;
        match command {
            DebugCommand::Status => self.debug.show_status(),
            DebugCommand::Start(name) => {
                self.debug.start(&self.editor, name.as_deref());
            }
            DebugCommand::Stop => {
                self.debug.stop();
            }
            DebugCommand::Continue => {
                self.debug.continue_execution();
            }
            DebugCommand::Next => {
                self.debug.step(StepKind::Next);
            }
            DebugCommand::StepIn => {
                self.debug.step(StepKind::In);
            }
            DebugCommand::StepOut => {
                self.debug.step(StepKind::Out);
            }
            DebugCommand::ToggleBreakpoint => {
                self.debug.toggle_breakpoint(&self.editor);
            }
            DebugCommand::Breakpoints => self.debug.show_breakpoints(),
            DebugCommand::Stack => {
                self.debug.stack_trace();
            }
            DebugCommand::Variables => {
                self.debug.variables();
            }
            DebugCommand::Output => self.debug.show_output(),
            DebugCommand::Configurations => self.debug.show_configurations(),
        }
        self.sync_debug()?;
        Ok(false)
    }

    pub(crate) fn sync_debug(&mut self) -> anyhow::Result<bool> {
        let changed = self.debug.poll_events();
        if let Some(message) = self.debug.take_message() {
            self.show_message(message);
        }
        if let Some(report) = self.debug.take_report() {
            self.show_help_text(report.topic, report.lines)?;
        }
        if let Some(jump) = self.debug.take_jump() {
            if let Err(message) =
                self.open_file_location(&jump.path, Some(jump.line), Some(jump.column))
            {
                self.show_message(message);
            }
        }
        Ok(changed)
    }
}

impl App {
    pub(crate) fn handle_debug_builtin(
        &mut self,
        command: crate::command::DebugBuiltin,
    ) -> anyhow::Result<bool> {
        use crate::command::DebugBuiltin;
        use kawi_editor::DebugCommand;
        let command = match command {
            DebugBuiltin::Start => DebugCommand::Start(None),
            DebugBuiltin::Stop => DebugCommand::Stop,
            DebugBuiltin::Continue => DebugCommand::Continue,
            DebugBuiltin::Next => DebugCommand::Next,
            DebugBuiltin::StepIn => DebugCommand::StepIn,
            DebugBuiltin::StepOut => DebugCommand::StepOut,
            DebugBuiltin::ToggleBreakpoint => DebugCommand::ToggleBreakpoint,
            DebugBuiltin::Stack => DebugCommand::Stack,
            DebugBuiltin::Variables => DebugCommand::Variables,
            DebugBuiltin::Status => DebugCommand::Status,
        };
        self.handle_debug_command(command)
    }
}
