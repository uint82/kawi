import json
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
            if "method" in message and "id" in message:
                self.write({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "error": {"code": -32601, "message": "method not found"},
                })


def initialize_result():
    return {
        "protocol_version": 1,
        "name": "trim",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": ["editor.get_state", "editor.apply_edit", "ui.message"],
    }


def trim_edits(state):
    lines = state.get("text", "").split("\n")
    edits = []
    for line_number, line in enumerate(lines):
        stripped = line.rstrip(" \t")
        if stripped == line:
            continue
        edits.append({
            "start": {"line": line_number, "byte_column": len(stripped.encode("utf-8"))},
            "end": {"line": line_number, "byte_column": len(line.encode("utf-8"))},
            "text": "",
        })
    return edits


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
            state = client.request("editor.get_state", {"include_text": True})
            if state is None:
                return
            if "error" in state:
                result = {"error": state["error"]}
            else:
                value = state["result"]
                edits = trim_edits(value)
                if not edits:
                    result = {"message": "no trailing whitespace"}
                else:
                    response = client.request("editor.apply_edit", {
                        "buffer": value["buffer"],
                        "revision": value["revision"],
                        "edits": edits,
                    })
                    if response is None:
                        return
                    result = response.get("result") or {"error": response.get("error")}
            client.write({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": result,
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
