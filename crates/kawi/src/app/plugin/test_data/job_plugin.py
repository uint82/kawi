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


def initialize_result():
    return {
        "protocol_version": 1,
        "name": "jobs",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": ["process.open", "process.signal", "ui.message"],
    }


def main():
    client = Client()
    output = []
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
            params = message.get("params", {})
            if params.get("command") == "run":
                reply = client.request("process.open", {
                    "command": "/bin/sh",
                    "args": ["-c", "printf hello"],
                })
                job = reply["result"]["job"] if reply and "result" in reply else None
                client.write({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "result": {"job": job},
                })
            elif params.get("command") == "signal":
                reply = client.request("process.open", {
                    "command": "/bin/sleep",
                    "args": ["5"],
                    "stdin": "null",
                })
                job = reply["result"]["job"] if reply and "result" in reply else None
                if job is not None:
                    client.request("process.signal", {
                        "job": job,
                        "signal": "terminate",
                    })
                client.write({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "result": {"job": job},
                })
            else:
                client.write({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "result": None,
                })
        elif method == "event":
            params = message.get("params", {})
            if params.get("event") == "job_output":
                output.append(params.get("payload", {}).get("data", ""))
            elif params.get("event") == "job_exited":
                payload = params.get("payload", {})
                text = "".join(output)
                message = f"{text}|{payload.get('state')}|{payload.get('exit_code')}"
                signal = payload.get("signal")
                if signal is not None:
                    message += f"|{signal}"
                client.request("ui.message", {"message": message})
                output.clear()
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
