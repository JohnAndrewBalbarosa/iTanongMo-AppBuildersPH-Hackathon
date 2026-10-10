"""Offline synthetic multilabel benchmark; no claim of field accuracy."""

import argparse
import ast
import hashlib
import importlib.metadata
import json
import math
import os
import platform
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ai-local"))
import triage


def classification_metrics(cases, predictions, labels):
    per_label = {}
    for label in labels:
        true_positive = false_positive = false_negative = support = 0
        for case, prediction in zip(cases, predictions, strict=True):
            expected = label in case["labels"]
            observed = label in prediction
            support += expected
            true_positive += expected and observed
            false_positive += not expected and observed
            false_negative += expected and not observed
        precision = (
            true_positive / (true_positive + false_positive)
            if true_positive + false_positive
            else 0
        )
        recall = (
            true_positive / (true_positive + false_negative)
            if true_positive + false_negative
            else 0
        )
        f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0
        per_label[label] = {
            "precision": precision,
            "recall": recall,
            "f1": f1,
            "support": support,
            "true_positive": true_positive,
            "false_positive": false_positive,
            "false_negative": false_negative,
        }
    return {
        "macro_f1": sum(item["f1"] for item in per_label.values()) / len(labels),
        "exact_match_rate": sum(
            set(case["labels"]) == set(prediction)
            for case, prediction in zip(cases, predictions, strict=True)
        )
        / len(cases),
        "per_label": per_label,
        "zero_division": 0,
    }


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def memory_peak():
    if sys.platform != "win32":
        return {"peak_working_set_bytes": None, "method": "Windows counter unavailable"}
    import ctypes
    from ctypes import wintypes

    class MemoryCounters(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("page_faults", wintypes.DWORD)] + [
            (name, ctypes.c_size_t)
            for name in [
                "peak_working_set",
                "working_set",
                "peak_paged_pool",
                "paged_pool",
                "peak_nonpaged_pool",
                "nonpaged_pool",
                "pagefile",
                "peak_pagefile",
            ]
        ]

    counters = MemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    ctypes.windll.kernel32.GetCurrentProcess.restype = wintypes.HANDLE
    ctypes.windll.psapi.GetProcessMemoryInfo.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(MemoryCounters),
        wintypes.DWORD,
    ]
    succeeded = ctypes.windll.psapi.GetProcessMemoryInfo(
        ctypes.windll.kernel32.GetCurrentProcess(), ctypes.byref(counters), counters.cb
    )
    if not succeeded:
        raise OSError("Process memory counter unavailable")
    return {
        "peak_working_set_bytes": counters.peak_working_set,
        "method": "Windows GetProcessMemoryInfo; entire benchmark process lifetime",
    }


def benchmark(fixture, output):
    dataset = json.loads(fixture.read_text(encoding="utf-8"))
    started = time.perf_counter()
    triage.verify_model()
    classifier = triage.load_classifier()
    load_ms = (time.perf_counter() - started) * 1000
    triage.classify(classifier, "Please provide the published annual program report.")
    predictions = []
    records = []
    durations = []
    repeated_equal = 0
    for case in dataset["cases"]:
        outputs = []
        for repetition in range(2):
            began = time.perf_counter()
            result = triage.classify(classifier, case["text"])
            duration_ms = (time.perf_counter() - began) * 1000
            scores = result["scores"]
            if (
                set(scores) != set(triage.CATEGORIES)
                or not all(
                    math.isfinite(score) and 0 <= score <= 1
                    for score in scores.values()
                )
                or result["human_review_required"] is not True
            ):
                raise ValueError("Invalid classifier output")
            prediction = sorted(
                label
                for label, score in scores.items()
                if score >= dataset["threshold"]
            )
            outputs.append(prediction)
            durations.append(duration_ms)
            records.append(
                {
                    "case_id": case["id"],
                    "repetition": repetition,
                    "predicted_labels": prediction,
                    "scores": scores,
                    "duration_ms": duration_ms,
                }
            )
        predictions.append(outputs[0])
        repeated_equal += outputs[0] == outputs[1]
    report = {
        "schema_version": "triage-benchmark/v1",
        "status": "measured",
        "verification": "unverified",
        "resume_eligible": False,
        "model": triage.MODEL_ID,
        "revision": triage.REVISION,
        "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
        "environment": {
            "python": platform.python_version(),
            "torch": importlib.metadata.version("torch"),
            "transformers": importlib.metadata.version("transformers"),
            "device": "cpu",
            "omp_threads": 4,
        },
        "workload": {
            "cases": len(dataset["cases"]),
            "repetitions": 2,
            "warmup_calls": 1,
            "threshold": dataset["threshold"],
            "offline": True,
        },
        "measurements": {
            **classification_metrics(dataset["cases"], predictions, triage.CATEGORIES),
            "label_agreement_repeated": repeated_equal / len(predictions),
            "load_and_integrity_check_ms": load_ms,
            "inference_p50_ms": percentile(durations, 0.5),
            "inference_p95_ms": percentile(durations, 0.95),
            "inference_p99_ms": percentile(durations, 0.99),
            **memory_peak(),
        },
        "records": records,
        "limitations": [
            "Synthetic labels are not independently verified; no real FOI ground truth",
            "Small constructed sample; no population accuracy claim or confidence interval",
            "Fixed 0.5 score threshold is uncalibrated; no post-test tuning",
            "Repeated calls in one run are not independent replication",
            "Load timing includes SHA-256 integrity verification; inference timing excludes it",
        ],
    }
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "event": "triage.benchmark.completed",
                "cases": len(predictions),
                "inference_calls": len(durations),
                "macro_f1": report["measurements"]["macro_f1"],
                "resume_eligible": False,
            }
        ),
        flush=True,
    )
    return classifier


def reuse_benchmark(cache_file, fixture, output):
    cache = json.loads(cache_file.read_text(encoding="utf-8"))
    source = Path(__file__).read_text(encoding="utf-8")
    tree = ast.parse(source)
    names = ["classification_metrics", "percentile", "memory_peak", "benchmark"]
    segments = [
        ast.get_source_segment(
            source,
            next(
                node
                for node in tree.body
                if isinstance(node, ast.FunctionDef) and node.name == name
            ),
        )
        for name in names
    ]
    algorithm_hash = hashlib.sha256("\n\n".join(segments).encode()).hexdigest()
    report = cache["benchmark"]
    compatible = (
        algorithm_hash == cache["algorithm_sha256"]
        and hashlib.sha256(
            (ROOT / "ai-local/triage.py").read_text(encoding="utf-8").encode()
        ).hexdigest()
        == cache["triage_source_canonical_sha256"]
        and json.loads(cache["original_artifact_utf8"]) == report
        and hashlib.sha256(cache["original_artifact_utf8"].encode()).hexdigest()
        == cache["original_artifact_sha256"]
        and hashlib.sha256(fixture.read_bytes()).hexdigest() == report["fixture_sha256"]
        and report["revision"] == triage.REVISION
        and report["environment"]["python"] == platform.python_version()
        and report["environment"]["torch"] == importlib.metadata.version("torch")
        and report["environment"]["transformers"]
        == importlib.metadata.version("transformers")
    )
    if not compatible:
        raise ValueError("Cached benchmark source, fixture, model or runtime mismatch")
    triage.verify_model()
    classifier = triage.load_classifier()
    report["measurement_provenance"] = {
        "reused": True,
        "original_run": cache["original_run"],
        "original_artifact_sha256": cache["original_artifact_sha256"],
        "producer_source_sha256": cache["producer_source_sha256"],
        "algorithm_sha256": algorithm_hash,
        "source_normalization": "CRLF normalized to LF; original producer byte hash retained",
        "checks": "same algorithm, triage source, fixture, pinned revision and runtime versions",
        "limitation": "Latency and peak memory belong to the original benchmark process, not this run",
    }
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {"event": "triage.benchmark.reused", "original_run": cache["original_run"]}
        ),
        flush=True,
    )
    return classifier


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path(__file__).parent / "fixtures/foi-cases.json",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--serve-port", type=int)
    parser.add_argument("--test-origin")
    parser.add_argument("--reuse-benchmark", type=Path)
    args = parser.parse_args()
    classifier = (
        reuse_benchmark(args.reuse_benchmark, args.fixture, args.output)
        if args.reuse_benchmark
        else benchmark(args.fixture, args.output)
    )
    if args.serve_port is not None:
        from urllib.parse import urlparse

        import companion

        origin = urlparse(args.test_origin or "")
        if origin.scheme != "http" or origin.hostname != "127.0.0.1":
            raise ValueError("Test companion accepts an isolated loopback origin only")
        companion.FRONTEND_ORIGIN = args.test_origin
        server = companion.create_server(classifier, port=args.serve_port)
        print(
            json.dumps(
                {
                    "event": "triage.test_server.ready",
                    "port": args.serve_port,
                    "pid": os.getpid(),
                }
            ),
            flush=True,
        )
        try:
            server.serve_forever()
        finally:
            server.server_close()
