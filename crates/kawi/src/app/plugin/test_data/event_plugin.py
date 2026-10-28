import json
import os
import sys


class Client:
    def __init__(self):
        self.next_id = 1

    def read(self):
        headers = {}
        while True:
            line = sys.stdin.buffer.readline()
            if not line:
                return None
            line = line.strip()
            if not line:
                break
            name, _, value = line.decode("ascii").partition(":")
            headers[name.lower()] = value.strip()
        length = int(headers["content-length"])
        return json.loads(sys.stdin.buffer.read(length))

    def write(self, message):
        body = json.dumps(message).encode("utf-8")
        sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
        sys.stdout.buffer.write(body)
        sys.stdout.buffer.flush()

    def request(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        self.write({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        while True:
            message = self.read()
            if message is None:
                return None
            if message.get("id") == request_id:
                return message


def initialize_result():
    return {
        "protocol_version": 1,
        "protocol_minor": 1,
        "name": "events",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": ["editor.get_state", "editor.apply_edit"],
        "events": ["buffer_changed"],
    }


def increment(path):
    count = 0
    if os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            count = int(handle.read().strip() or "0")
    count += 1
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(str(count))
    return count


def append_marker(client):
    state = client.request("editor.get_state", {"include_text": True})
    if state is None or "result" not in state:
        return
    value = state["result"]
    text = value["text"]
    lines = text.split("\n")
    edit = {
        "start": {"line": len(lines) - 1, "byte_column": len(lines[-1].encode("utf-8"))},
        "end": {"line": len(lines) - 1, "byte_column": len(lines[-1].encode("utf-8"))},
        "text": "!",
    }
    client.request("editor.apply_edit", {
        "buffer": value["buffer"],
        "revision": value["revision"],
        "edits": [edit],
    })


def log_event(payload):
    with open("events.log", "a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload) + "\n")


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "log"
    client = Client()
    while True:
        message = client.read()
        if message is None:
            return
        method = message.get("method")
        if method == "initialize":
            client.write({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": initialize_result(),
            })
        elif method == "event":
            params = message.get("params", {})
            if params.get("event") != "buffer_changed":
                continue
            if mode == "log":
                log_event(params.get("payload"))
            else:
                count = increment("event-edit.count")
                if count > 3:
                    return
                append_marker(client)
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
