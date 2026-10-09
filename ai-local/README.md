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
