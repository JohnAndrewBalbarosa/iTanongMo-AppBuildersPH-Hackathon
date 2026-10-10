"""Isolated SQLite fixtures and online-backup recovery measurements."""

import argparse
import hashlib
import json
import sqlite3
from pathlib import Path


def prepare_government(database):
    with sqlite3.connect(database) as connection:
        connection.execute(
            "UPDATE auth_users SET persona_id='dost-decision' "
            "WHERE email='officer.judge@itanongmo.local'"
        )
        output = json.dumps(
            {
                "body": "Verified scholarship statistics are in the official annual reports.",
                "subject": "Scholarship records",
                "source_record_ids": [],
                "limitations": ["Synthetic test fixture; verify report period"],
                "requires_human_review": True,
            }
        )
        connection.execute(
            "INSERT INTO ai_jobs(id,request_id,task_kind,status,input_json,input_hash,"
            "output_json,output_hash,prompt_version,available_at,created_at) VALUES "
            "('evidence-job','auth-demo-request-review','government_response',"
            "'completed','{}','evidence-input',?,'evidence-hash','test',"
            "datetime('now'),datetime('now'))",
            (output,),
        )


def record_fingerprint(database):
    with sqlite3.connect(f"file:{database.as_posix()}?mode=ro", uri=True) as connection:
        tables = ["foi_requests", "messages", "ai_jobs", "ai_draft_reviews"]
        counts = {}
        digest = hashlib.sha256()
        for table in tables:
            rows = connection.execute(f"SELECT * FROM {table} ORDER BY id").fetchall()
            counts[table] = len(rows)
            digest.update(json.dumps(rows, separators=(",", ":")).encode())
        return {"counts": counts, "record_sha256": digest.hexdigest()}


def snapshot(database, destination):
    with sqlite3.connect(database) as source, sqlite3.connect(destination) as target:
        source.backup(target)
    return record_fingerprint(destination)


def recover(database, destination):
    return snapshot(database, destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action", choices=["prepare", "snapshot", "recover", "fingerprint"]
    )
    parser.add_argument("database", type=Path)
    parser.add_argument("destination", type=Path, nargs="?")
    args = parser.parse_args()
    if args.action == "prepare":
        prepare_government(args.database)
        result = {"fixture": "synthetic_completed_draft", "outcome": "prepared"}
    elif args.action == "fingerprint":
        result = record_fingerprint(args.database)
    else:
        if (
            args.destination is None
            or args.destination.resolve() == args.database.resolve()
        ):
            raise ValueError(
                "Recovery destination must be a separate isolated database"
            )
        result = (snapshot if args.action == "snapshot" else recover)(
            args.database, args.destination
        )
    print(json.dumps(result))


if __name__ == "__main__":
    main()
