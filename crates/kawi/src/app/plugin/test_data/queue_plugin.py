import json
import sys
import time


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


def wait_for(host_id):
    while True:
        reply = read_message()
        if reply is None or reply.get("id") == host_id:
            return


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
                    "name": "queue",
                    "version": "0.1.0",
                    "plugin_capabilities": ["invoke_command"],
                    "host_capabilities": ["ui.message"],
                },
            })
        elif method == "invoke_command":
            params = message.get("params", {})
            args = params.get("args") or {}
            sleep_ms = int(args.get("sleep_ms") or 0)
            if sleep_ms > 0:
                time.sleep(sleep_ms / 1000)
            if args.get("crash"):
                sys.exit(1)
            if args.get("report"):
                revision = (params.get("context") or {}).get("revision", 0)
                host_id = 900
                write_message({
                    "jsonrpc": "2.0",
                    "id": host_id,
                    "method": "ui.message",
                    "params": {"message": f"revision:{revision}"},
                })
                wait_for(host_id)
            write_message({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": {"ok": True},
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
