use super::*;
use crate::app::App;

impl App {
    pub(crate) fn handle_debug_command(
        &mut self,
        command: kawi_editor::DebugCommand,
    ) -> anyhow::Result<bool> {
        use kawi_editor::DebugCommand;
        match command {
            command @ (DebugCommand::Status
            | DebugCommand::Start(_)
            | DebugCommand::Restart
            | DebugCommand::Stop
            | DebugCommand::Continue
            | DebugCommand::Next
            | DebugCommand::StepIn
            | DebugCommand::StepOut
            | DebugCommand::Configurations) => self.handle_debug_session_command(command),
            command @ (DebugCommand::ToggleBreakpoint
            | DebugCommand::BreakpointCondition(_)
            | DebugCommand::BreakpointHitCondition(_)
            | DebugCommand::BreakpointLogMessage(_)
            | DebugCommand::BreakpointClear
            | DebugCommand::Breakpoints
            | DebugCommand::FunctionBreakpoint(_)
            | DebugCommand::FunctionBreakpoints
            | DebugCommand::Exceptions
            | DebugCommand::ToggleException(_)) => self.handle_debug_breakpoint_command(command),
            command @ (DebugCommand::Stack
            | DebugCommand::Variables
            | DebugCommand::ExpandVariables(_)
            | DebugCommand::Threads
            | DebugCommand::Thread(_)) => self.handle_debug_inspection_command(command),
            command @ (DebugCommand::Evaluate(_)
            | DebugCommand::Console
            | DebugCommand::Watch(_)
            | DebugCommand::EditWatch(_, _)
            | DebugCommand::Unwatch(_)
            | DebugCommand::Watches
            | DebugCommand::Output) => self.handle_debug_console_command(command),
        }
        self.sync_debug()?;
        Ok(false)
    }

    fn handle_debug_session_command(&mut self, command: kawi_editor::DebugCommand) {
        use kawi_editor::DebugCommand;
        match command {
            DebugCommand::Status => self.debug.show_status(),
            DebugCommand::Start(name) => {
                self.debug.start(&self.editor, name.as_deref());
            }
            DebugCommand::Restart => {
                self.debug.restart(&self.editor);
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
            DebugCommand::Configurations => self.debug.show_configurations(),
            _ => {}
        }
    }

    fn handle_debug_breakpoint_command(&mut self, command: kawi_editor::DebugCommand) {
        use kawi_editor::DebugCommand;
        match command {
            DebugCommand::ToggleBreakpoint => {
                self.debug.toggle_breakpoint(&self.editor);
            }
            DebugCommand::BreakpointCondition(condition) => {
                self.debug.set_breakpoint_condition(&self.editor, condition);
            }
            DebugCommand::BreakpointHitCondition(condition) => {
                self.debug
                    .set_breakpoint_hit_condition(&self.editor, condition);
            }
            DebugCommand::BreakpointLogMessage(message) => {
                self.debug.set_breakpoint_log_message(&self.editor, message);
            }
            DebugCommand::BreakpointClear => {
                self.debug.clear_breakpoint_options(&self.editor);
            }
            DebugCommand::Breakpoints => self.debug.show_breakpoints(),
            DebugCommand::FunctionBreakpoint(name) => {
                self.debug.toggle_function_breakpoint(name);
            }
            DebugCommand::FunctionBreakpoints => self.debug.show_function_breakpoints(),
            DebugCommand::Exceptions => self.debug.show_exception_filters(),
            DebugCommand::ToggleException(filter) => {
                self.debug.toggle_exception_filter(filter);
            }
            _ => {}
        }
    }

    fn handle_debug_inspection_command(&mut self, command: kawi_editor::DebugCommand) {
        use kawi_editor::DebugCommand;
        match command {
            DebugCommand::Stack => {
                self.debug.stack_trace();
            }
            DebugCommand::Variables => {
                self.debug.variables();
            }
            DebugCommand::ExpandVariables(reference) => {
                self.debug.expand_variables(reference);
            }
            DebugCommand::Threads => {
                self.debug.threads();
            }
            DebugCommand::Thread(thread_id) => {
                self.debug.select_thread(thread_id);
            }
            _ => {}
        }
    }

    fn handle_debug_console_command(&mut self, command: kawi_editor::DebugCommand) {
        use kawi_editor::DebugCommand;
        match command {
            DebugCommand::Evaluate(expression) => {
                self.debug.evaluate(expression);
            }
            DebugCommand::Console => self.debug.show_console(),
            DebugCommand::Watch(expression) => {
                self.debug.add_watch(expression);
            }
            DebugCommand::EditWatch(index, expression) => {
                self.debug.edit_watch(index, expression);
            }
            DebugCommand::Unwatch(expression) => {
                self.debug.remove_watch(&expression);
            }
            DebugCommand::Watches => self.debug.show_watches(),
            DebugCommand::Output => self.debug.show_output(),
            _ => {}
        }
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
            DebugBuiltin::Restart => DebugCommand::Restart,
            DebugBuiltin::Stop => DebugCommand::Stop,
            DebugBuiltin::Continue => DebugCommand::Continue,
            DebugBuiltin::Next => DebugCommand::Next,
            DebugBuiltin::StepIn => DebugCommand::StepIn,
            DebugBuiltin::StepOut => DebugCommand::StepOut,
            DebugBuiltin::ToggleBreakpoint => DebugCommand::ToggleBreakpoint,
            DebugBuiltin::Stack => DebugCommand::Stack,
            DebugBuiltin::Variables => DebugCommand::Variables,
            DebugBuiltin::Threads => DebugCommand::Threads,
            DebugBuiltin::Functions => DebugCommand::FunctionBreakpoints,
            DebugBuiltin::Exceptions => DebugCommand::Exceptions,
            DebugBuiltin::Console => DebugCommand::Console,
            DebugBuiltin::Watches => DebugCommand::Watches,
            DebugBuiltin::Status => DebugCommand::Status,
        };
        self.handle_debug_command(command)
    }
}
