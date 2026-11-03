use super::*;
use crate::app::App;
use kawi_dap::RunInTerminalRequest;

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
            | DebugCommand::Terminate
            | DebugCommand::Panel
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
            | DebugCommand::MoreVariables(_)
            | DebugCommand::SetVariable { .. }
            | DebugCommand::Frame(_)
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
        self.sync_terminal_panel_size();
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
            DebugCommand::Terminate => {
                self.debug.terminate();
            }
            DebugCommand::Panel => self.toggle_debug_panel(),
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
            DebugCommand::MoreVariables(reference) => {
                self.debug.load_more_variables(reference);
            }
            DebugCommand::SetVariable {
                reference,
                name,
                value,
            } => {
                self.debug.set_variable(reference, name, value);
            }
            DebugCommand::Frame(frame_id) => {
                self.debug.select_frame(frame_id);
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
        self.handle_run_in_terminal();
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

    fn handle_run_in_terminal(&mut self) {
        let Some(request) = self.debug.take_run_in_terminal() else {
            return;
        };
        let request_seq = request.request_seq;
        match self.open_run_in_terminal(&request) {
            Ok(process_id) => {
                self.debug
                    .respond_run_in_terminal(request_seq, Some(process_id), None);
            }
            Err(message) => {
                self.debug
                    .respond_run_in_terminal(request_seq, None, Some(message));
            }
        }
    }

    fn open_run_in_terminal(&mut self, request: &RunInTerminalRequest) -> Result<u32, String> {
        if request.kind.as_deref() == Some("external") {
            return Err("external terminals are not supported".to_string());
        }
        if request.args.is_empty() {
            return Err("runInTerminal has no command".to_string());
        }
        let command = terminal_command(request);
        let cwd = request.cwd.clone().unwrap_or_else(|| self.project_root());
        self.open_debug_terminal(command, cwd, request.env.clone())
            .ok_or_else(|| "failed to start debug terminal".to_string())
    }
}

fn terminal_command(request: &RunInTerminalRequest) -> String {
    if request.args_can_be_interpreted_by_shell {
        return request.args.join(" ");
    }
    request
        .args
        .iter()
        .map(|argument| shell_quote(argument))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(argument: &str) -> String {
    if argument
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_./:=@".contains(character))
    {
        return argument.to_string();
    }
    format!("'{}'", argument.replace('\'', "'\\''"))
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
            DebugBuiltin::Terminate => DebugCommand::Terminate,
            DebugBuiltin::Panel => DebugCommand::Panel,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn request(args: Vec<&str>, args_can_be_interpreted_by_shell: bool) -> RunInTerminalRequest {
        RunInTerminalRequest {
            request_seq: 1,
            kind: None,
            title: None,
            cwd: None,
            args: args.into_iter().map(str::to_string).collect(),
            env: BTreeMap::new(),
            args_can_be_interpreted_by_shell,
        }
    }

    #[test]
    fn quotes_terminal_arguments() {
        assert_eq!(
            terminal_command(&request(vec!["printf", "debug-terminal"], false)),
            "printf debug-terminal"
        );
        assert_eq!(
            terminal_command(&request(vec!["printf", "hello world"], false)),
            "printf 'hello world'"
        );
        assert_eq!(
            terminal_command(&request(vec!["printf", "it's"], false)),
            "printf 'it'\\''s'"
        );
    }

    #[test]
    fn keeps_shell_arguments_when_requested() {
        assert_eq!(
            terminal_command(&request(vec!["printf", "$HOME"], true)),
            "printf $HOME"
        );
    }
}
