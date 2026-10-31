import json
import sys
import time
from pathlib import Path

RETRY_MARKER = Path(__file__).with_name(".retry-once")
RETRY_HANG_SECONDS = 120


def read_message():
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


def write_message(message):
    body = json.dumps(message).encode("utf-8")
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
    sys.stdout.buffer.write(body)
    sys.stdout.buffer.flush()


def request_host(method, params, request_id):
    write_message({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params,
    })
    while True:
        reply = read_message()
        if reply is None:
            return False
        if reply.get("id") == request_id:
            return True


def exercise_retry():
    if RETRY_MARKER.exists():
        RETRY_MARKER.unlink()
        return {"message": "recovered after timeout"}
    RETRY_MARKER.write_text("", encoding="ascii")
    time.sleep(RETRY_HANG_SECONDS)
    return None


def main():
    while True:
        message = read_message()
        if message is None:
            return
        method = message.get("method")
        if method == "initialize":
            write_message({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": {
                    "protocol_version": 1,
                    "name": "echo",
                    "version": "0.1.0",
                    "plugin_capabilities": ["invoke_command"],
                    "host_capabilities": ["editor.get_state", "editor.apply_edit", "ui.message"],
                },
            })
        elif method == "invoke_command":
            params = message.get("params", {})
            command = params.get("command")
            name = (params.get("args") or {}).get("name", "world")
            if command == "hello":
                if not request_host("ui.message", {"message": f"pong {name}"}, 900):
                    return
            elif command == "retry":
                result = exercise_retry()
                if result is None:
                    return
                if not request_host("ui.message", {"message": result["message"]}, 901):
                    return
                write_message({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "result": result,
                })
                continue
            write_message({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": {"message": f"pong {name}"},
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
