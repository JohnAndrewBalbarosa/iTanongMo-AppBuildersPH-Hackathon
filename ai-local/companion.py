"""Windows local-model companion for the signed-in government portal."""

import argparse
import json
import os
import time
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, HTTPServer

from triage import (
    MODEL_ID,
    MODEL_PATH,
    REVISION,
    classify,
    download_model,
    load_classifier,
    verify_model,
)

FRONTEND_ORIGIN = "https://johnandrewbalbarosa.github.io"
HOST = "127.0.0.1"
PORT = 8765
MAX_BODY_BYTES = 16_384
MAX_TEXT_LENGTH = 8_000


def main():
    # Mental model: acquire and verify the pinned model once, then serve local-only advisory inference.
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", action="store_true", help="Verify offline inference and exit")
    args = parser.parse_args()
    started = time.monotonic()
    classifier = prepare_classifier()
    if args.smoke:
        result = classify(classifier, "Please reconsider the denial of my request.")
        emit("local_model.smoke.completed", "success", started, categories=len(result["scores"]))
        return
    server = create_server(classifier)
    emit("local_model.server.started", "success", started, host=HOST, port=PORT)
    try:
        server.serve_forever(poll_interval=0.5)
    except KeyboardInterrupt:
        emit("local_model.server.stopped", "success", time.monotonic())
    finally:
        server.server_close()


def prepare_classifier():
    started = time.monotonic()
    try:
        verify_model(MODEL_PATH)
    except FileNotFoundError:
        emit("local_model.download.started", "started", started, revision=REVISION)
        download_model(MODEL_PATH)
        verify_model(MODEL_PATH)
        emit("local_model.download.completed", "success", started, revision=REVISION)
    os.environ["HF_HUB_OFFLINE"] = "1"
    classifier = load_classifier(MODEL_PATH)
    emit("local_model.model.ready", "success", started, model=MODEL_ID)
    return classifier


def create_server(classifier, port=PORT):
    return HTTPServer((HOST, port), make_handler(classifier))


def make_handler(classifier):
    class Handler(BaseHTTPRequestHandler):
        def do_OPTIONS(self):
            if not self.authorized_origin() or not self.authorized_host() or self.path not in ("/health", "/classify"):
                self.respond(403, {"error": "Forbidden."})
                return
            self.respond(204, None)

        def do_GET(self):
            if not self.authorized_host() or self.path != "/health":
                self.respond(404, {"error": "Not found."})
                return
            if self.headers.get("Origin") and not self.authorized_origin():
                self.respond(403, {"error": "Forbidden."})
                return
            self.respond(200, {"ready": True, "model": MODEL_ID})

        def do_POST(self):
            started = time.monotonic()
            if not self.authorized_host() or not self.authorized_origin() or self.path != "/classify":
                self.respond(403, {"error": "Forbidden."})
                return
            if self.headers.get("Content-Type", "").split(";", 1)[0].strip() != "application/json":
                self.respond(415, {"error": "JSON is required."})
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
            except ValueError:
                length = 0
            if length < 1 or length > MAX_BODY_BYTES:
                self.respond(413, {"error": "Request is too large or empty."})
                return
            try:
                body = json.loads(self.rfile.read(length))
                value = body.get("text") if isinstance(body, dict) else None
                if not isinstance(value, str) or not 1 <= len(value.strip()) <= MAX_TEXT_LENGTH:
                    raise ValueError("Invalid request text.")
            except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
                self.respond(400, {"error": "Invalid request text."})
                return
            try:
                result = classify(classifier, value)
                self.respond(200, result)
                emit("local_model.classify.completed", "success", started, text_length=len(value))
            except Exception as error:  # noqa: BLE001 - isolate model failures from the browser.
                emit("local_model.classify.failed", "error", started, error_type=type(error).__name__)
                self.respond(500, {"error": "Local inference failed. Restart the EXE and retry."})

        def authorized_origin(self):
            return self.headers.get("Origin") == FRONTEND_ORIGIN

        def authorized_host(self):
            return self.headers.get("Host") == f"{HOST}:{self.server.server_port}"

        def respond(self, status, payload):
            content = json.dumps(payload).encode("utf-8") if payload is not None else b""
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(content)))
            self.send_header("Cache-Control", "no-store")
            if self.authorized_origin():
                self.send_header("Access-Control-Allow-Origin", FRONTEND_ORIGIN)
                self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                self.send_header("Access-Control-Allow-Headers", "content-type")
                self.send_header("Access-Control-Allow-Private-Network", "true")
                self.send_header("Vary", "Origin")
            self.end_headers()
            if content:
                self.wfile.write(content)

        def log_message(self, _format, *_args):
            # Avoid recording request data or browser headers in access logs.
            return

    return Handler


def emit(event, outcome, started, **details):
    print(json.dumps({
        "time": datetime.now(UTC).isoformat(),
        "severity": "error" if outcome == "error" else "info",
        "component": "local-model-companion",
        "event": event,
        "outcome": outcome,
        "duration_ms": round((time.monotonic() - started) * 1000),
        **details,
    }), flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        emit("local_model.start.failed", "error", time.monotonic(), error_type=type(error).__name__)
        raise
