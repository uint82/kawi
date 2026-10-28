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


def write_result(result):
    with open("sandbox_result.json", "w", encoding="utf-8") as handle:
        json.dump(result, handle)


def main():
    with open("sandbox.pid", "w", encoding="utf-8") as handle:
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
                "name": "sandbox",
                "version": "0.1.0",
                "plugin_capabilities": ["invoke_command"],
                "host_capabilities": [],
                "events": [],
            })
        elif method == "invoke_command":
            path = message["params"]["args"]
            try:
                with open(path, encoding="utf-8") as source:
                    result = {"ok": True, "content": source.read()}
            except OSError as error:
                result = {"ok": False, "error": str(error), "errno": error.errno}
            write_result(result)
            respond(message, result)
        elif method == "shutdown":
            return


if __name__ == "__main__":
    main()
