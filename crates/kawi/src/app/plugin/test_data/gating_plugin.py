import json
import sys


class Client:
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


def append(path, line):
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(line + "\n")


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
                    "name": "gating",
                    "version": "0.1.0",
                    "plugin_capabilities": ["invoke_command"],
                    "host_capabilities": [],
                    "events": ["buffer_changed", "buffer_opened", "file_opened", "file_saved"],
                },
            })
        elif method == "event":
            params = message.get("params", {})
            append("gated.log", params.get("event", ""))
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
