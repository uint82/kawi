import json
import os
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


def respond(request, result):
    write_message({"jsonrpc": "2.0", "id": request.get("id"), "result": result})


def operate(params):
    args = params.get("args") or {}
    operation = args.get("operation")
    path = args.get("path")
    try:
        if operation == "read":
            with open(path, "r", encoding="utf-8") as handle:
                return {"ok": True, "content": handle.read()}
        if operation == "prefix":
            return {"ok": True, "prefix": sys.prefix}
        if operation == "write":
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("written")
            return {"ok": True}
        return {"ok": False, "error": f"unknown operation {operation!r}"}
    except OSError as error:
        return {"ok": False, "error": str(error), "errno": error.errno}


def main():
    with open("sandbox_probe.pid", "w", encoding="utf-8") as handle:
        handle.write(str(os.getpid()))
    while True:
        message = read_message()
        if message is None:
            return
        method = message.get("method")
        if method == "initialize":
            respond(message, {
                "protocol_version": 1,
                "protocol_minor": 2,
                "name": "python-probe",
                "version": "0.1.0",
                "plugin_capabilities": ["invoke_command"],
                "host_capabilities": [],
                "events": [],
            })
        elif method == "invoke_command":
            respond(message, operate(message.get("params") or {}))
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
