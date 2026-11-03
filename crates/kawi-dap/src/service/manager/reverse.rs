use super::*;

impl Manager {
    pub(super) fn handle_reverse_request(
        &mut self,
        request_seq: i64,
        command: String,
        arguments: Option<Value>,
    ) {
        if command == "runInTerminal" {
            let request = super::parse::run_in_terminal(request_seq, arguments.as_ref());
            self.reverse_requests.insert(request_seq, command);
            self.emit(DapEvent::RunInTerminal(request));
            return;
        }
        let message = if command == "startDebugging" {
            "startDebugging is not supported"
        } else {
            "unsupported request"
        };
        self.send_response(request_seq, false, command, Some(message.to_string()), None);
    }

    pub(super) fn respond_run_in_terminal(
        &mut self,
        request_seq: i64,
        process_id: Option<u32>,
        message: Option<String>,
    ) {
        let Some(command) = self.reverse_requests.remove(&request_seq) else {
            return;
        };
        let success = process_id.is_some();
        let body = process_id.map(|process_id| serde_json::json!({"processId": process_id}));
        let message = if success {
            None
        } else {
            Some(message.unwrap_or_else(|| "runInTerminal failed".to_string()))
        };
        self.send_response(request_seq, success, command, message, body);
    }
}
