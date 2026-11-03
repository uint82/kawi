use anyhow::Result;

use kawi_editor::LspCommand;

use super::lsp::editor_path;
use super::App;

#[derive(Clone, Copy)]
enum LspLifecycle {
    Start,
    Stop,
    Restart,
}

impl LspLifecycle {
    fn gerund(self) -> &'static str {
        match self {
            Self::Start => "Starting",
            Self::Stop => "Stopping",
            Self::Restart => "Restarting",
        }
    }
}

impl App {
    pub(super) fn handle_lsp_command(&mut self, command: LspCommand) -> Result<bool> {
        match command {
            LspCommand::Status => self.show_lsp_status(),
            LspCommand::Start(server) => self.request_lsp_lifecycle(LspLifecycle::Start, server),
            LspCommand::Stop(server) => self.request_lsp_lifecycle(LspLifecycle::Stop, server),
            LspCommand::Restart(server) => {
                self.request_lsp_lifecycle(LspLifecycle::Restart, server)
            }
        }
    }

    pub(super) fn start_lsp(&mut self) -> Result<bool> {
        self.request_lsp_lifecycle(LspLifecycle::Start, None)
    }

    pub(super) fn stop_lsp(&mut self) -> Result<bool> {
        self.request_lsp_lifecycle(LspLifecycle::Stop, None)
    }

    pub(super) fn restart_lsp(&mut self) -> Result<bool> {
        self.request_lsp_lifecycle(LspLifecycle::Restart, None)
    }

    fn request_lsp_lifecycle(
        &mut self,
        action: LspLifecycle,
        server: Option<String>,
    ) -> Result<bool> {
        let path = editor_path(&self.editor);
        if path.is_none() && server.is_none() {
            self.show_message("E: LSP command requires a file or server".to_string());
            return Ok(false);
        }
        let server = server.as_deref();
        let sent = match action {
            LspLifecycle::Start => self.lsp.start(path.as_deref(), server),
            LspLifecycle::Stop => self.lsp.stop(path.as_deref(), server),
            LspLifecycle::Restart => self.lsp.restart(path.as_deref(), server),
        };
        if sent {
            self.show_message(lifecycle_message(action, server));
        } else {
            self.show_message("E: Language server service is not running".to_string());
        }
        Ok(false)
    }
}

fn lifecycle_message(action: LspLifecycle, server: Option<&str>) -> String {
    match server {
        Some(server) => format!("{} language server {server}", action.gerund()),
        None => format!("{} language server", action.gerund()),
    }
}

#[cfg(test)]
mod tests;
