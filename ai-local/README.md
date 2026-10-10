# Local triage experiment

CPU-only multilingual mDeBERTa NLI classifier; no API key or GPU required.
Model: `MoritzLaurer/mDeBERTa-v3-base-mnli-xnli` (MIT), pinned to
`8adb042d524ecd5c26d3e3ba0e3fbcf7e2d0864c`.
Source: https://huggingface.co/MoritzLaurer/mDeBERTa-v3-base-mnli-xnli

From the project root:

```powershell
uv sync --project ai-local --python 3.12
uv run --project ai-local python ai-local/triage.py --download
uv run --project ai-local python ai-local/triage.py --smoke
uv run --project ai-local python ai-local/triage.py --text "Kailangan agad ang report bago ang deadline bukas."
```

Artifacts: `var/models/mdeberta-triage`; environment: `ai-local/.venv`.
After download, model loading and inference use local files only.
Scores cover the eight existing `src/triage/rules.rs` categories independently;
they are not calibrated probabilities or automatic routing decisions.
Smoke tests check execution and score validity, not FOI classification accuracy.
Tagalog/Taglish accuracy requires evaluation on labeled local examples.
The Rust service still uses its existing provider; this experiment is standalone.
Long input may be truncated by the tokenizer; chunking is not implemented.

## Windows government companion

`companion.py` connects the signed-in `government-demo.html` page to the same
pinned mDeBERTa classifier. The Windows `iTanongMoLocalModel.exe` bundles Python
and its runtime dependencies; the user downloads only the EXE. A browser cannot
launch a downloaded program, so the user opens it once. On first launch it
downloads the model to `%LOCALAPPDATA%\iTanongMo\models\mdeberta-triage`, checks
the pinned files' SHA-256 hashes, and starts an HTTP service bound only to
`127.0.0.1:8765`. Later launches use those files without a network download.
The user keeps the EXE open while using the portal; Chrome may request local
network permission. Model scores are advisory and never save triage automatically.
The local classification endpoint accepts only this Pages origin and JSON
requests within its size limit. Request text is not logged or sent to the Rust
API by this feature.

Build on Windows from the project root:

```powershell
uv sync --project ai-local --python 3.12
powershell -File ai-local/build-exe.ps1
```

The ignored output is `var/local-model-build/dist/iTanongMoLocalModel.exe`.
PyInstaller is pinned to 6.22.3 in the build script; this pilot packaging choice
remains untested on other Windows machines. Before release, run the EXE with
`ITANONGMO_MODEL_DIR` set to the verified local model directory and `--smoke`.
The public download is a GitHub Release asset, not a Git-tracked Pages file.
