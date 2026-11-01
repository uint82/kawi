import json
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


def initialize_result():
    return {
        "protocol_version": 1,
        "protocol_minor": 5,
        "name": "ui",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": ["ui.input", "ui.progress_start", "ui.progress_update", "ui.progress_finish"],
        "events": [],
    }


def log_response(payload):
    with open("ui-response.log", "w", encoding="utf-8") as handle:
        handle.write(json.dumps(payload))


def run_progress(client):
    response = client.request("ui.progress_start", {
        "title": "Work",
        "message": "running",
        "fraction": 0.0,
    })
    if not response or "result" not in response:
        return {"canceled": True}
    progress_id = response["result"]["id"]
    canceled = False
    for step in range(100):
        time.sleep(0.02)
        response = client.request("ui.progress_update", {
            "id": progress_id,
            "message": "running",
            "fraction": step / 100.0,
        })
        canceled = bool(response and response.get("result", {}).get("canceled"))
        if canceled:
            break
    client.request("ui.progress_finish", {"id": progress_id})
    return {"canceled": canceled}


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
                "result": initialize_result(),
            })
        elif method == "invoke_command":
            command = message.get("params", {}).get("command")
            if command == "progress":
                log_response(run_progress(client))
            else:
                response = client.request("ui.input", {
                    "title": "Name",
                    "placeholder": None,
                    "value": None,
                    "max_length": 32,
                })
                log_response(response.get("result") if response else None)
            client.write({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": {"ok": True},
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
