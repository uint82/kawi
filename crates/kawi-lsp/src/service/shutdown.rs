use super::*;
use crate::client::ClientEvent;
use crate::protocol::{Message, RequestId};
use std::time::{Duration, Instant};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(500);
const EXIT_TIMEOUT: Duration = Duration::from_millis(250);

struct ShutdownClient {
    key: ClientKey,
    client: ClientState,
    request: Option<RequestId>,
}

impl Manager {
    pub(super) fn shutdown_all(&mut self) {
        let mut clients = self.drain_clients();
        self.send_shutdown_requests(&mut clients);
        self.wait_for_shutdown_responses(&mut clients);
        self.finish_clients(clients);
    }

    pub(super) fn shutdown_client(&mut self, key: &ClientKey, client: &mut ClientState) {
        let Some(request) = self.send_shutdown_request(client) else {
            return;
        };
        let _ = self.wait_for_shutdown_response(key, &request);
        send_exit(client);
        let _ = client.client.wait_timeout(EXIT_TIMEOUT);
        client.client.terminate();
    }

    fn drain_clients(&mut self) -> Vec<ShutdownClient> {
        self.clients
            .drain()
            .map(|(key, client)| ShutdownClient {
                key,
                client,
                request: None,
            })
            .collect()
    }

    fn send_shutdown_requests(&mut self, clients: &mut [ShutdownClient]) {
        for entry in clients {
            entry.request = self.send_shutdown_request(&entry.client);
        }
    }

    fn send_shutdown_request(&mut self, client: &ClientState) -> Option<RequestId> {
        let id = self.request_id();
        client
            .client
            .send(Message::request(id.clone(), "shutdown", None))
            .then_some(id)
    }

    fn wait_for_shutdown_response(&mut self, key: &ClientKey, request: &RequestId) -> bool {
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            match self.input.recv_timeout(remaining) {
                Ok(ServiceInput::Client(ClientEvent::Message {
                    key: event_key,
                    message: Message::Response { id, .. },
                    ..
                })) if event_key == *key && id == *request => return true,
                Ok(ServiceInput::Client(ClientEvent::ReaderClosed { key: event_key, .. }))
                    if event_key == *key =>
                {
                    return false
                }
                Ok(_) => {}
                Err(_) => return false,
            }
        }
    }

    fn wait_for_shutdown_responses(&mut self, clients: &mut [ShutdownClient]) {
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        while clients.iter().any(|entry| entry.request.is_some()) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            match self.input.recv_timeout(remaining) {
                Ok(ServiceInput::Client(ClientEvent::Message {
                    key,
                    message: Message::Response { id, .. },
                    ..
                })) => mark_responded(clients, &key, &id),
                Ok(ServiceInput::Client(ClientEvent::ReaderClosed { key, .. })) => {
                    mark_client_stopped(clients, &key)
                }
                Ok(_) => {}
                Err(_) => return,
            }
        }
    }

    fn finish_clients(&mut self, clients: Vec<ShutdownClient>) {
        let deadline = Instant::now() + EXIT_TIMEOUT;
        for entry in &clients {
            send_exit(&entry.client);
        }
        for mut entry in clients {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !remaining.is_zero() {
                let _ = entry.client.client.wait_timeout(remaining);
            }
            entry.client.client.terminate();
            self.emit_status(&entry.key.server, ServerState::Stopped, None);
        }
    }
}

fn send_exit(client: &ClientState) {
    let _ = client.client.send(Message::notification("exit", None));
}

fn mark_responded(clients: &mut [ShutdownClient], key: &ClientKey, id: &RequestId) {
    if let Some(entry) = clients
        .iter_mut()
        .find(|entry| entry.key == *key && entry.request.as_ref() == Some(id))
    {
        entry.request = None;
    }
}

fn mark_client_stopped(clients: &mut [ShutdownClient], key: &ClientKey) {
    if let Some(entry) = clients.iter_mut().find(|entry| entry.key == *key) {
        entry.request = None;
    }
}
