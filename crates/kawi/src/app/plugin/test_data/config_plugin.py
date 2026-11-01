import json
import sys


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
            return None
        if reply.get("id") == request_id:
            if "error" in reply:
                raise RuntimeError(reply["error"]["message"])
            return reply.get("result")


def setting_text(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def handle(command, args):
    if command == "read":
        key = args.get("key", "size")
        result = request_host("config.get", {"key": key}, 901)
        value = result.get("value") if result else None
        request_host("ui.message", {"message": setting_text(value)}, 902)
    elif command == "list":
        result = request_host("config.list", None, 903)
        items = result.get("settings", []) if result else []
        text = " ".join(f"{item['key']}={setting_text(item['value'])}" for item in items)
        request_host("ui.message", {"message": text}, 904)
    elif command == "execute":
        request_host("editor.execute", {"command": args.get("command", "mode.insert")}, 905)
        request_host("ui.message", {"message": "executed"}, 906)
    elif command == "commands":
        result = request_host("editor.list_commands", None, 907)
        commands = result.get("commands", []) if result else []
        text = " ".join(item["id"] for item in commands)
        request_host("ui.message", {"message": text}, 908)
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
                    "name": "configurable",
                    "version": "0.1.0",
                    "plugin_capabilities": ["invoke_command"],
                    "host_capabilities": [
                        "config.get",
                        "config.list",
                        "editor.execute",
                        "editor.list_commands",
                        "ui.message",
                    ],
                },
            })
        elif method == "invoke_command":
            params = message.get("params", {})
            args = params.get("args") or {}
            try:
                result = handle(params.get("command"), args)
            except Exception as error:
                write_message({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "error": {"code": -32000, "message": str(error)},
                })
                continue
            write_message({
                "jsonrpc": "2.0",
                "id": message["id"],
                "result": result,
            })
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
