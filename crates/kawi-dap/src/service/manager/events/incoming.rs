use super::*;

impl Manager {
    pub(super) fn handle_event(&mut self, event: &str, body: Option<&Value>) {
        match event {
            "initialized" => self.handle_initialized_event(),
            "stopped" => self.handle_stopped_event(body),
            "continued" => self.handle_continued_event(body),
            "exited" => self.handle_exited_event(body),
            "terminated" => self.finish_session(DebugStatus::Terminated, None),
            "output" => self.handle_output_event(body),
            "breakpoint" => self.handle_breakpoint_event(body),
            _ => {}
        }
    }

    fn handle_initialized_event(&mut self) {
        if self.stopping {
            return;
        }
        self.initialized = true;
        self.send_launch();
        self.send_all_breakpoints();
        self.send_function_breakpoints();
        self.send_exception_breakpoints();
        self.send_configuration_done();
    }

    fn handle_stopped_event(&mut self, body: Option<&Value>) {
        self.emit(DapEvent::Status(DebugStatus::Stopped));
        self.emit(DapEvent::Stopped {
            reason: string_field(body, "reason").unwrap_or_else(|| "pause".to_string()),
            thread_id: body
                .and_then(|body| body.get("threadId"))
                .and_then(Value::as_i64),
            all_threads_stopped: body
                .and_then(|body| body.get("allThreadsStopped"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            description: string_field(body, "description"),
            text: string_field(body, "text"),
        });
    }

    fn handle_continued_event(&mut self, body: Option<&Value>) {
        let thread_id = body
            .and_then(|body| body.get("threadId"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let all_threads_continued = body
            .and_then(|body| body.get("allThreadsContinued"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.emit(DapEvent::Status(DebugStatus::Running));
        self.emit(DapEvent::Continued {
            thread_id,
            all_threads_continued,
        });
    }

    fn handle_exited_event(&mut self, body: Option<&Value>) {
        self.emit(DapEvent::Exited {
            exit_code: body
                .and_then(|body| body.get("exitCode"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
        });
    }

    fn handle_output_event(&mut self, body: Option<&Value>) {
        self.emit(DapEvent::Output {
            category: string_field(body, "category").unwrap_or_else(|| "console".to_string()),
            output: string_field(body, "output").unwrap_or_default(),
        });
    }

    fn handle_breakpoint_event(&mut self, body: Option<&Value>) {
        let breakpoint = body
            .and_then(|body| body.get("breakpoint"))
            .map(super::super::parse::breakpoint);
        if let Some(breakpoint) = breakpoint {
            self.emit(DapEvent::Breakpoint {
                reason: string_field(body, "reason").unwrap_or_else(|| "changed".to_string()),
                breakpoint,
            });
        }
    }
}

fn string_field(body: Option<&Value>, key: &str) -> Option<String> {
    body.and_then(|body| body.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}
