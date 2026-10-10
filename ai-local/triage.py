"""Offline CPU triage experiment; scores are suggestions requiring human review."""

import argparse
import hashlib
import json
import os
import sys
import time
from pathlib import Path

os.environ.setdefault("HF_HUB_DISABLE_PROGRESS_BARS", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")
os.environ.setdefault("OMP_NUM_THREADS", "4")
os.environ.setdefault("MKL_NUM_THREADS", "4")

MODEL_ID = "MoritzLaurer/mDeBERTa-v3-base-mnli-xnli"
REVISION = "8adb042d524ecd5c26d3e3ba0e3fbcf7e2d0864c"
MODEL_PATH = Path(os.environ.get("ITANONGMO_MODEL_DIR") or (
    Path(os.environ.get("LOCALAPPDATA") or Path.home() / "AppData/Local")
    / "iTanongMo/models/mdeberta-triage"
    if getattr(sys, "frozen", False)
    else Path(__file__).resolve().parents[1] / "var/models/mdeberta-triage"
))
MODEL_FILES = {
    "added_tokens.json": "fb697283833d25e2c711f1bc37730ecd8b20f4bd5f015db1d84aefe0adc9155a",
    "config.json": "4e4430c95100d613df80fa01276f231931e1b535557cf62fbb3cc50323e50cca",
    "model.safetensors": "65af59b1ff4450b09ecbf13ca35c840dbf038b26ff8e10e5ea89ca724828ed1e",
    "special_tokens_map.json": "9463f61e1b109a8eb4688b829260d7c6b1e6dff04c98ff7269bb89e2b92369b9",
    "spm.model": "13c8d666d62a7bc4ac8f040aab68e942c861f93303156cc28f5c7e885d86d6e3",
    "tokenizer_config.json": "df9fd3a4482cc4a866788244824573fab8e74dadd1609d4f0e5997f9e96921dc",
    "tokenizer.json": "3aca3ce69a0a35aeb144a52c4f1d41c4246b8785f8f398315cc8fb6b24057810",
}
CATEGORIES = {
    "time_sensitive": "a time-sensitive request with an urgent deadline",
    "safety_review": "a request concerning danger, threats or public safety",
    "sensitive_data": "a request for personal or confidential information",
    "voluminous": "a request for a large volume of records",
    "multi_unit": "a request involving multiple government agencies or units",
    "appeal": "an appeal against a denied information request",
    "records_preservation": "a request to preserve records against destruction",
    "clarification": "a vague request that needs clarification",
}
SMOKE_CASES = [
    ("Please reconsider the denial of my information request.", "appeal"),
    ("Humihingi ako ng medical records at personal na tirahan.", "sensitive_data"),
    ("Kailangan agad ang report bago ang deadline bukas.", "time_sensitive"),
]


def main():
    # Mental model: acquire pinned artifacts, or load offline and score labels.
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--download", action="store_true")
    action.add_argument("--smoke", action="store_true")
    action.add_argument(
        "--text", help="Request text; keep sensitive data out of shell history"
    )
    args = parser.parse_args()
    started = time.monotonic()
    if args.download:
        download_model()
        verify_model()
        emit("model.download.completed", started, revision=REVISION)
        return
    verify_model()
    classifier = load_classifier()
    emit("model.load.completed", started, offline=True, device="cpu")
    if args.smoke:
        run_smoke(classifier)
    else:
        print(json.dumps(classify(classifier, args.text), ensure_ascii=False))


def download_model(model_path=MODEL_PATH):
    from huggingface_hub import snapshot_download

    snapshot_download(
        MODEL_ID,
        revision=REVISION,
        local_dir=model_path,
        allow_patterns=["*.json", "*.txt", "*.model", "*.safetensors", "README.md"],
        max_workers=2,
    )


def verify_model(model_path=MODEL_PATH):
    for name, expected in MODEL_FILES.items():
        file = model_path / name
        if not file.is_file():
            raise FileNotFoundError(f"Missing model file: {name}")
        with file.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        if digest != expected:
            raise ValueError(f"Model file failed SHA-256 verification: {name}")


def load_classifier(model_path=MODEL_PATH):
    from transformers import AutoModelForSequenceClassification, AutoTokenizer, pipeline

    tokenizer = AutoTokenizer.from_pretrained(model_path, local_files_only=True)
    model = AutoModelForSequenceClassification.from_pretrained(
        model_path, local_files_only=True, use_safetensors=True
    )
    return pipeline(
        "zero-shot-classification", model=model, tokenizer=tokenizer, device=-1
    )


def classify(classifier, text):
    if not text or not text.strip():
        raise ValueError("Request text must not be empty")
    result = classifier(
        text,
        candidate_labels=list(CATEGORIES.values()),
        hypothesis_template="This text is {}.",
        multi_label=True,
    )
    category_ids = {description: key for key, description in CATEGORIES.items()}
    return {
        "model": MODEL_ID,
        "revision": REVISION,
        "human_review_required": True,
        "scores": {
            category_ids[label]: round(score, 4)
            for label, score in zip(result["labels"], result["scores"])
        },
    }


def run_smoke(classifier):
    for index, (text, expected) in enumerate(SMOKE_CASES):
        started = time.monotonic()
        result = classify(classifier, text)
        scores = result["scores"]
        assert len(scores) == len(CATEGORIES) and all(
            0 <= v <= 1 for v in scores.values()
        )
        emit(
            "triage.smoke.completed",
            started,
            case=index,
            expected=expected,
            top_category=next(iter(scores)),
            expected_score=scores[expected],
        )
    print(
        json.dumps(
            {
                "event": "triage.smoke.passed",
                "cases": len(SMOKE_CASES),
                "accuracy_validated": False,
            }
        )
    )


def emit(event, started, **details):
    print(
        json.dumps(
            {
                "event": event,
                "model": MODEL_ID,
                "duration_ms": round((time.monotonic() - started) * 1000),
                **details,
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(
            json.dumps({"event": "triage.failed", "error_type": type(error).__name__})
        )
        raise
