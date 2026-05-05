"""Post new fines to Slack."""
# pylint: disable=missing-function-docstring

from __future__ import annotations

import csv
import os
import tempfile
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import requests
from dotenv import load_dotenv
from slack_sdk import WebClient

DEFAULT_API_BASE = "http://localhost:3000"
DEFAULT_STATE_FILE = ".fines_notifier_state.txt"
TIMEOUT_SECONDS = 20
FINES_CHANNEL_ID = "C0B1X3W6MHS"

CSV_FIELDNAMES = [
    "ysws",
    "approved_at",
    "code_url",
    "demo_url",
    "hours",
    "country",
    "description",
    "github_username",
    "display_name",
    "archived_demo",
    "archived_repo",
    "airtable_id",
]


def required_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"Missing required environment variable: {name}")
    return value


def load_last_sent_id(path: Path) -> int:
    if not path.exists():
        return 0

    try:
        return int(path.read_text(encoding="utf-8").strip())
    except ValueError:
        return 0


def save_last_sent_id(path: Path, last_id: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"{last_id}\n", encoding="utf-8")


def fetch_fines(api_base: str) -> list[dict[str, Any]]:
    url = f"{api_base.rstrip('/')}/api/v1/fines"
    response = requests.get(url, timeout=TIMEOUT_SECONDS)
    response.raise_for_status()

    payload = response.json()
    if not isinstance(payload, list):
        raise RuntimeError("Unexpected fines response: expected a JSON array")

    return payload


def fine_id(fine: dict[str, Any]) -> int:
    return int(fine["id"])


def amount_dollars(amount_cents: int | None) -> str:
    return f"${(amount_cents or 0) / 100:.2f}"


def timestamp_to_day(timestamp: int | None) -> str:
    if timestamp is None:
        return "unknown"
    dt = datetime.fromtimestamp(timestamp, tz=UTC)
    return dt.strftime("%Y-%m-%d")


def deleted_project_rows_for_fine(fine: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "ysws": project.get("ysws"),
            "approved_at": timestamp_to_day(project.get("approved_at")),
            "code_url": project.get("code_url"),
            "demo_url": project.get("demo_url"),
            "hours": project.get("true_hours") or project.get("hours"),
            "country": project.get("country"),
            "description": project.get("description"),
            "github_username": project.get("github_username"),
            "display_name": project.get("display_name"),
            "archived_demo": project.get("archived_demo"),
            "archived_repo": project.get("archived_repo"),
            "airtable_id": project.get("airtable_id"),
        }
        for project in fine.get("projects") or []
    ]


def write_csv(rows: list[dict[str, Any]]) -> Path:
    tmp = tempfile.NamedTemporaryFile(
        mode="w", newline="", encoding="utf-8", suffix=".csv", delete=False
    )

    with tmp:
        writer = csv.DictWriter(tmp, fieldnames=CSV_FIELDNAMES)
        writer.writeheader()
        writer.writerows(rows)

    return Path(tmp.name)


def upload_file_with_comment(
    token: str,
    channel_id: str,
    fine: dict[str, Any],
    csv_path: Path,
    timeout_s: int,
    thread_ts: str | None = None,
) -> None:
    timestamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%SZ")
    filename = f"deleted-projects-fine-{fine_id(fine)}-{timestamp}.csv"

    comment_lines = [
        f"*New fine: ID {fine_id(fine)}*",
        f"Amount: {amount_dollars(fine.get('amount_cents'))}",
        f"YSWS: {fine.get('ysws') or 'unknown'}",
        f"Date: {fine.get('date') or 'unknown'}",
        f"Deleted projects: {len(fine.get('projects') or [])}",
    ]
    transaction_id = fine.get("transaction_id")
    if transaction_id:
        comment_lines.append(
            f"<https://hcbscan.3kh0.net/app/txn/{transaction_id}|Transaction>"
        )

    client = WebClient(token=token, timeout=timeout_s)
    params: dict[str, Any] = {
        "channel": channel_id,
        "filename": filename,
        "title": filename,
        "initial_comment": "\n".join(comment_lines),
        "file": str(csv_path),
    }
    if thread_ts:
        params["thread_ts"] = thread_ts

    resp = client.files_upload_v2(**params)
    if not resp.get("ok"):
        raise RuntimeError(f"Slack API files_upload_v2 failed: {resp}")


def main() -> int:
    load_dotenv()

    slack_token = required_env("SLACK_BOT_TOKEN")
    api_base = os.environ.get("OTTER_API_BASE", DEFAULT_API_BASE).rstrip("/")
    state_file = Path(os.environ.get("FINES_STATE_FILE", DEFAULT_STATE_FILE))

    last_sent_id = load_last_sent_id(state_file)

    fines = fetch_fines(api_base)
    new_fines = [fine for fine in fines if fine_id(fine) > last_sent_id]

    if not new_fines:
        print("No new fines found; nothing posted.")
        return 0

    for fine in sorted(new_fines, key=fine_id):
        csv_path = write_csv(deleted_project_rows_for_fine(fine))
        try:
            upload_file_with_comment(
                slack_token,
                FINES_CHANNEL_ID,
                fine,
                csv_path,
                timeout_s=TIMEOUT_SECONDS,
            )
        finally:
            csv_path.unlink(missing_ok=True)

    save_last_sent_id(state_file, max(fine_id(fine) for fine in new_fines))

    print(f"Posted {len(new_fines)} new fine(s).")
    return 0


if __name__ == "__main__":
    main()
