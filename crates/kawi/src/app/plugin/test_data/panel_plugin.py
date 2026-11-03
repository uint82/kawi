import json
import sys


class Client:
    def __init__(self):
        self.next_id = 1
        self.pending = []

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
            if message.get("method"):
                self.pending.append(message)

    def next_message(self):
        if self.pending:
            return self.pending.pop(0)
        return self.read()


PANEL = None


def initialize_result():
    return {
        "protocol_version": 1,
        "name": "panels",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": [
            "ui.panel_open",
            "ui.panel_update",
            "ui.panel_focus",
            "ui.panel_close",
            "ui.message",
        ],
    }


def open_panel(client):
    global PANEL
    reply = client.request("ui.panel_open", {
        "title": "Index",
        "status": "building",
        "height": 6,
        "focus": False,
        "lines": [
            {"text": "alpha", "style": "success"},
            {"text": "beta"},
        ],
    })
    PANEL = reply["result"]["panel"]
    client.request("ui.message", {"message": f"opened:{PANEL}"})


def update_panel(client):
    client.request("ui.panel_update", {
        "panel": PANEL,
        "status": "complete",
        "lines": [{"text": "gamma"}],
    })
    client.request("ui.message", {"message": "updated"})


def focus_panel(client):
    client.request("ui.panel_focus", {"panel": PANEL})
    client.request("ui.message", {"message": "focused"})


def close_panel(client):
    client.request("ui.panel_close", {"panel": PANEL})
    client.request("ui.message", {"message": "closed"})


def main():
    client = Client()
    while True:
        message = client.next_message()
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
            if command == "open":
                open_panel(client)
            elif command == "update":
                update_panel(client)
            elif command == "focus":
                focus_panel(client)
            elif command == "close":
                close_panel(client)
            client.write({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": None,
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
