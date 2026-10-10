"""End-to-end artifact checks plus offline recording contract tests."""

import json
import re
import subprocess
import unittest
from itertools import pairwise

import imageio_ffmpeg
from fixtures import REFERENCE, REPLY, RecordingAPI
from PIL import Image
from story import DURATION, FPS, NARRATION, OUTPUT, SCENES, SIZE, WORK, timestamp


class ShowcaseTests(unittest.TestCase):
    def test_scene_and_speech_timeline(self):
        self.assertEqual(SCENES[0][0], 0)
        self.assertEqual(SCENES[-1][1], DURATION)
        for current, following in pairwise(SCENES):
            self.assertEqual(current[1], following[0])
        for index, (start, end, words) in enumerate(NARRATION):
            self.assertLess(start, end)
            self.assertLessEqual(end, DURATION)
            self.assertLessEqual(len(words.split()) / (end - start), 3)
            if index:
                self.assertGreaterEqual(start - NARRATION[index - 1][1], 1)
            scene = next(scene for scene in SCENES if scene[0] <= start < scene[1])
            self.assertLessEqual(end, scene[1])

    def test_staged_request_and_response_flow(self):
        api = RecordingAPI()
        self.assertEqual(api.respond("/citizen/requests", "GET", {})["requests"], [])
        result = api.respond("/submissions", "POST", {})
        self.assertEqual(result["requests"][0]["reference"], REFERENCE)
        self.assertEqual(
            len(api.respond("/citizen/requests", "GET", {})["requests"]), 1
        )
        api.respond(
            f"/government/requests/{REFERENCE}/messages", "POST", {"body": REPLY}
        )
        detail = api.respond(f"/citizen/requests/{REFERENCE}", "GET", {})
        self.assertEqual(detail["request"]["status"], "in_review")
        self.assertEqual(detail["timeline"]["messages"][0]["body"], REPLY)
        with self.assertRaises(ValueError):
            api.respond("/unexpected", "GET", {})

    def test_recording_has_no_frontend_errors(self):
        capture = json.loads((WORK / "capture.json").read_text(encoding="utf-8"))
        self.assertEqual(capture["page_errors"], [])
        self.assertTrue(capture["submitted"])
        self.assertTrue(capture["replied"])
        self.assertEqual(capture["live_api_calls"], 0)
        for name in ("title", "details", "reply"):
            metadata = json.loads((WORK / f"{name}.json").read_text(encoding="utf-8"))
            self.assertGreater(metadata["frames"], 20)
            self.assertGreaterEqual(metadata["field"]["y"], 80)
            self.assertLessEqual(
                metadata["field"]["y"] + metadata["field"]["height"], 810
            )
            self.assertTrue((WORK / f"{name}-{metadata['frames'] - 1:03}.png").exists())
        for name in ("home", "submitted", "board", "tracking"):
            with Image.open(WORK / f"{name}.png") as image:
                self.assertEqual(image.size, (1440, 810))

    def test_srt_matches_narration_exactly(self):
        cues = (
            (OUTPUT / "itanongmo-showcase.srt")
            .read_text(encoding="utf-8")
            .strip()
            .split("\n\n")
        )
        self.assertEqual(len(cues), len(NARRATION))
        for index, (cue, (start, end, words)) in enumerate(
            zip(cues, NARRATION, strict=True), 1
        ):
            self.assertEqual(
                cue.splitlines(),
                [str(index), f"{timestamp(start)} --> {timestamp(end)}", words],
            )

    def test_final_video_decodes_to_end(self):
        video = OUTPUT / "itanongmo-showcase.mp4"
        self.assertGreater(video.stat().st_size, 100000)
        # Decode both streams in full; inspect only a bounded metadata record afterward.
        executable = imageio_ffmpeg.get_ffmpeg_exe()
        result = subprocess.run(
            [
                executable,
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                str(video),
                "-map",
                "0:v:0",
                "-map",
                "0:a:0",
                "-f",
                "null",
                "-",
            ],
            capture_output=True,
            check=False,
            timeout=180,
        )
        self.assertEqual(
            result.returncode, 0, result.stderr.decode(errors="replace")[-1200:]
        )
        reader = imageio_ffmpeg.read_frames(str(video), pix_fmt="rgb24")
        try:
            metadata = next(reader)
        finally:
            reader.close()
        self.assertEqual(metadata["size"], SIZE)
        self.assertAlmostEqual(metadata["fps"], FPS, places=2)
        self.assertAlmostEqual(metadata["duration"], DURATION, delta=0.1)
        atoms = video.read_bytes()
        self.assertLess(atoms.find(b"moov"), atoms.find(b"mdat"))
        self.assertIn(b"avc1", atoms)
        self.assertIn(b"mp4a", atoms)
        self.assertIsNone(re.search(rb"dummy data", atoms, re.IGNORECASE))


if __name__ == "__main__":
    unittest.main()
