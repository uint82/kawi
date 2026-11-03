use super::*;
use crate::client::ClientEvent;
use crate::protocol::Message;
use std::time::{Duration, Instant};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

impl Manager {
    pub(super) fn shutdown_all(&mut self) {
        let clients: Vec<(ClientKey, ClientState)> = self.clients.drain().collect();
        for (key, mut client) in clients {
            self.shutdown_client(&key, &mut client);
            client.client.terminate();
            self.emit_status(&key.server, ServerState::Stopped, None);
        }
    }

    fn shutdown_client(&mut self, key: &ClientKey, client: &mut ClientState) {
        let id = self.request_id();
        if !client
            .client
            .send(Message::request(id.clone(), "shutdown", None))
        {
            return;
        }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self.input.recv_timeout(remaining) {
                Ok(ServiceInput::Client(ClientEvent::Message {
                    key: event_key,
                    message:
                        Message::Response {
                            id: response_id, ..
                        },
                    ..
                })) if event_key == *key && response_id == id => break,
                Ok(ServiceInput::Client(ClientEvent::ReaderClosed { key: event_key, .. }))
                    if event_key == *key =>
                {
                    return
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        client.client.send(Message::notification("exit", None));
        let _ = client.client.wait_timeout(SHUTDOWN_TIMEOUT);
    }
}
