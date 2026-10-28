import json
import os
import sys
import time


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


def wait_for(path):
    while not os.path.exists(path):
        time.sleep(0.01)


def main():
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
                "result": {
                    "protocol_version": 1,
                    "protocol_minor": 1,
                    "name": "inflight",
                    "version": "0.1.0",
                    "plugin_capabilities": ["invoke_command"],
                    "host_capabilities": ["editor.apply_edit"],
                    "events": [],
                },
            })
        elif method == "invoke_command":
            with open("inflight.ready", "w", encoding="utf-8") as handle:
                handle.write("ready")
            wait_for("inflight.go")
            response = client.request("editor.apply_edit", {
                "buffer": 0,
                "revision": 0,
                "edits": [],
            })
            with open("inflight.log", "w", encoding="utf-8") as handle:
                json.dump(response, handle)
            client.write({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": {"ok": True},
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
