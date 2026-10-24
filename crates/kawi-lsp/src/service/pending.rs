use super::*;
use crate::protocol::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReplyPurpose {
    Navigation,
    Rename,
    Text,
    CompletionResolve,
}

pub(super) struct PendingReply {
    pub(super) request: u64,
    pub(super) key: ClientKey,
    pub(super) encoding: PositionEncodingKind,
    pub(super) purpose: ReplyPurpose,
}

impl Manager {
    pub(super) fn send_tracked_request(
        &mut self,
        request: u64,
        key: &ClientKey,
        method: &str,
        params: Option<Value>,
        purpose: ReplyPurpose,
    ) {
        let id = self.request_id();
        let encoding = self
            .clients
            .get(key)
            .map_or(PositionEncodingKind::UTF16, |client| {
                client.encoding.clone()
            });
        self.pending_replies.insert(
            id.clone(),
            PendingReply {
                request,
                key: key.clone(),
                encoding,
                purpose,
            },
        );
        let Some(params) = params else {
            self.fail_reply(&id, "request params could not be serialized".to_string());
            return;
        };
        let sent = self.clients.get(key).is_some_and(|client| {
            client
                .client
                .send(Message::request(id.clone(), method, Some(params)))
        });
        if !sent {
            self.fail_reply(&id, "server is not accepting messages".to_string());
        }
    }

    pub(super) fn cancel_request(&mut self, request: u64) {
        let navigation = self.pending_requests.remove(&request).is_some();
        let rename = self.pending_renames.remove(&request).is_some();
        let text = self.pending_text_requests.remove(&request).is_some();
        let resolve = self.pending_completion_resolves.remove(&request).is_some();
        if !navigation && !rename && !text && !resolve {
            return;
        }
        let ids: Vec<RequestId> = self
            .pending_replies
            .iter()
            .filter(|(_, reply)| reply.request == request)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            let Some(reply) = self.pending_replies.remove(&id) else {
                continue;
            };
            let Some(client) = self.clients.get(&reply.key) else {
                continue;
            };
            client.client.send(Message::notification(
                "$/cancelRequest",
                Some(serde_json::json!({"id": id.to_value()})),
            ));
        }
    }

    pub(super) fn fail_reply(&mut self, id: &RequestId, message: String) {
        let Some(reply) = self.pending_replies.remove(id) else {
            return;
        };
        match reply.purpose {
            ReplyPurpose::Navigation => self.fail_navigation_reply(reply.request, message),
            ReplyPurpose::Rename => self.fail_rename_request(reply.request, message),
            ReplyPurpose::Text => self.fail_text_request(reply.request, message),
            ReplyPurpose::CompletionResolve => {
                self.pending_completion_resolves.remove(&reply.request);
                self.emit_text_error(reply.request, message);
            }
        }
    }

    pub(super) fn fail_requests_for_client(&mut self, key: &ClientKey, message: &str) {
        let ids: Vec<RequestId> = self
            .pending_replies
            .iter()
            .filter(|(_, reply)| &reply.key == key)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.fail_reply(&id, message.to_string());
        }
        let requests: Vec<u64> = self
            .pending_renames
            .iter()
            .filter(|(_, rename)| &rename.key == key)
            .map(|(request, _)| *request)
            .collect();
        for request in requests {
            self.fail_rename_request(request, message.to_string());
        }
    }
}
