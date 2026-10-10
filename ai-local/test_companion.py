"""Security boundary and model-integrity tests for the Windows companion."""

import hashlib
import http.client
import json
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import companion

import triage


class CompanionHttpTests(unittest.TestCase):
    def setUp(self):
        self.server = companion.create_server(object(), port=0)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    def request(self, method, path, body=None, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=3)
        connection.request(method, path, body=body, headers=headers or {})
        response = connection.getresponse()
        payload = response.read()
        result = response.status, dict(response.getheaders()), json.loads(payload) if payload else None
        connection.close()
        return result

    def test_health_and_preflight_allow_only_the_pages_origin(self):
        allowed = {"Origin": companion.FRONTEND_ORIGIN}
        status, headers, payload = self.request("GET", "/health", headers=allowed)
        self.assertEqual(status, 200)
        self.assertTrue(payload["ready"])
        self.assertEqual(headers["Access-Control-Allow-Origin"], companion.FRONTEND_ORIGIN)
        status, headers, _ = self.request("OPTIONS", "/classify", headers=allowed)
        self.assertEqual(status, 204)
        self.assertEqual(headers["Access-Control-Allow-Private-Network"], "true")
        status, _, _ = self.request("OPTIONS", "/health", headers=allowed)
        self.assertEqual(status, 204)
        status, headers, _ = self.request("OPTIONS", "/classify", headers={"Origin": "https://evil.test"})
        self.assertEqual(status, 403)
        self.assertNotIn("Access-Control-Allow-Origin", headers)

    def test_rejects_host_spoofing_and_cross_origin_post(self):
        body = b'{"text":"private request"}'
        allowed = {"Origin": companion.FRONTEND_ORIGIN, "Content-Type": "application/json"}
        status, _, _ = self.request("POST", "/classify", body, {**allowed, "Host": "evil.test"})
        self.assertEqual(status, 403)
        status, _, _ = self.request("POST", "/classify", body, {"Content-Type": "application/json"})
        self.assertEqual(status, 403)

    def test_rejects_invalid_or_oversize_input_before_inference(self):
        allowed = {"Origin": companion.FRONTEND_ORIGIN, "Content-Type": "application/json"}
        with patch.object(companion, "classify") as classify:
            status, _, _ = self.request("POST", "/classify", b'{"text":""}', allowed)
            self.assertEqual(status, 400)
            status, _, _ = self.request("POST", "/classify", b"x" * (companion.MAX_BODY_BYTES + 1), allowed)
            self.assertEqual(status, 413)
            classify.assert_not_called()

    def test_returns_review_only_scores_without_saving_request_text(self):
        allowed = {"Origin": companion.FRONTEND_ORIGIN, "Content-Type": "application/json"}
        with patch.object(companion, "classify", return_value={"scores": {"appeal": 0.7}}) as classify:
            status, headers, payload = self.request("POST", "/classify", b'{"text":"Review the appeal"}', allowed)
            self.assertEqual(status, 200)
            self.assertEqual(payload["scores"], {"appeal": 0.7})
            self.assertEqual(headers["Cache-Control"], "no-store")
            classify.assert_called_once_with(unittest.mock.ANY, "Review the appeal")


class ModelIntegrityTests(unittest.TestCase):
    def test_first_launch_downloads_then_verifies_before_loading(self):
        with (
            patch.object(companion, "verify_model", side_effect=[FileNotFoundError(), None]) as verify,
            patch.object(companion, "download_model") as download,
            patch.object(companion, "load_classifier", return_value="ready") as load,
        ):
            self.assertEqual(companion.prepare_classifier(), "ready")
            self.assertEqual(verify.call_count, 2)
            download.assert_called_once_with(companion.MODEL_PATH)
            load.assert_called_once_with(companion.MODEL_PATH)

    def test_rejects_modified_model_file(self):
        with tempfile.TemporaryDirectory() as directory:
            model_file = Path(directory) / "sample.bin"
            model_file.write_bytes(b"expected")
            expected = hashlib.sha256(b"expected").hexdigest()
            with patch.dict(triage.MODEL_FILES, {"sample.bin": expected}, clear=True):
                triage.verify_model(Path(directory))
                model_file.write_bytes(b"modified")
                with self.assertRaisesRegex(ValueError, "SHA-256"):
                    triage.verify_model(Path(directory))


if __name__ == "__main__":
    unittest.main()
