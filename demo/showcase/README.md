# iTanongMo showcase film

90-second, 1920×1080, 24 fps MP4 with an original quiet ambient score, actual frontend captures, request/response typing, and a matching Tagalog narration SRT. No voice recording is included: record narration against the SRT windows. Cues leave 1–2 seconds of silence and longer screen holds. No paid API, GPU, or live backend is required.

## Deliverables

- `output/itanongmo-showcase.mp4`: H.264/AAC, browser-compatible, fast-start.
- `output/itanongmo-showcase.srt`: narration timestamps; load beside the MP4 in a player/editor.
- `output/narration.md`: timed speaking script.
- `output/poster.jpg`, `output/storyboard.jpg`: title frame and six-scene preview.

## Production context

The request, agency queue, timeline events, and government response are staged synthetic fixtures, not evidence of live transactions or agency participation. The recording uses the real repository frontend with Playwright request interception; the backend may remain offline. Fixture responses and recording configuration exist only inside the recording browser. The deployed frontend and backend are not modified. On-screen copy has no dummy-data label; this production note preserves provenance for editors. No statement claims completed live service or actual government turnaround.

Recording-only CSS expands the agency board to full width to counter the current homepage section rule. The citizen consent label uses “privacy notice” in place of “demo privacy notice” only in the capture browser. Dates, request identifiers, and `alex@example.org` are synthetic. No credentials or real citizen data are used. No live login, request submission, or government reply occurs.

## Rebuild (Windows, Chrome installed)

From the repository root:

```powershell
uv venv var/showcase/env
uv pip install --python var/showcase/env/Scripts/python.exe -r demo/showcase/requirements.txt
var/showcase/env/Scripts/python.exe -m ruff check demo/showcase
var/showcase/env/Scripts/python.exe -m compileall -q demo/showcase
var/showcase/env/Scripts/python.exe demo/showcase/render.py
var/showcase/env/Scripts/python.exe demo/showcase/verify.py
```

Use `render.py --reuse-captures` to re-encode existing captures without opening the frontend again. Temporary captures, WAV, encoder logs, and the intermediate video are under ignored `var/showcase/`. Final artifacts are under `demo/showcase/output/`. The renderer streams frames and keeps six screenshots cached; it does not load the whole movie into RAM. Rendering uses two encoder threads. The soundtrack is synthesized locally and can be muted under narration.

## Sources and selection

- [Playwright screenshots](https://playwright.dev/python/docs/screenshots) and [official Python repository](https://github.com/microsoft/playwright-python): capture actual local frontend pixels and intercept fixture API responses. Apache-2.0.
- [imageio-ffmpeg](https://github.com/imageio/imageio-ffmpeg): BSD-2-Clause wrapper distributes an FFmpeg executable through platform wheels; used only to resolve the executable, encoding uses the documented FFmpeg CLI. See the bundled FFmpeg build's license for binary redistribution.
- [FFmpeg CLI](https://ffmpeg.org/ffmpeg.html): encode H.264 and mux AAC with `+faststart`.
- [Pillow](https://github.com/python-pillow/Pillow): screenshot compositing and typography. HPND.
- [NumPy](https://github.com/numpy/numpy): local original soundtrack synthesis and background gradients. BSD-3-Clause.

Pinned tool versions are in `requirements.txt`. These are tooling dependencies isolated from the application. No existing video renderer was found in the repository. The installed observability CLI lacks `tech`/`reuse` commands; the source and selection record is kept here. Windows Segoe UI fonts and installed Chrome are local rendering prerequisites; neither is copied into deliverables.
