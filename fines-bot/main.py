"""Posts YSWS fines to Slack"""

from __future__ import annotations

import argparse
import csv
import os
import sqlite3
import tempfile
import time
import traceback
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import requests
from dotenv import load_dotenv
from slack_sdk import WebClient

HCB_TRANSACTIONS_URL = (
    "https://hcb.hackclub.com/api/v3/organizations/org_NOuVez/transactions"
)
DEFAULT_API_BASE = "http://localhost:3000"
DEFAULT_INTERVAL_SECONDS = 60 * 60
DEFAULT_LEADERBOARD_INTERVAL_SECONDS = 24 * 60 * 60
DEFAULT_DB_PATH = Path(__file__).resolve().parent / "fines-bot.sqlite3"
TIMEOUT_SECONDS = 20
PAGE_SIZE = 100
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

LEADERBOARD_FIELDNAMES = [
    "ysws",
    "total_dollars",
    "change_dollars",
]


def required_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"Missing required environment variable: {name}")
    return value


def connect_db(path: Path) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path)
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS sent_fines (
            transaction_id TEXT PRIMARY KEY,
            amount_cents INTEGER NOT NULL,
            memo TEXT NOT NULL,
            hcb_date TEXT NOT NULL,
            posted_at TEXT NOT NULL,
            slack_ts TEXT
        )
        """
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS leaderboard_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            posted_at INTEGER NOT NULL
        )
        """
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS leaderboard_snapshot (
            ysws TEXT PRIMARY KEY,
            total_cents INTEGER NOT NULL
        )
        """
    )
    return conn


def sent_ids(conn: sqlite3.Connection) -> set[str]:
    return {
        row[0]
        for row in conn.execute("SELECT transaction_id FROM sent_fines").fetchall()
    }


def mark_sent(
    conn: sqlite3.Connection, transaction: dict[str, Any], slack_ts: str | None
) -> None:
    conn.execute(
        """
        INSERT OR REPLACE INTO sent_fines
        (transaction_id, amount_cents, memo, hcb_date, posted_at, slack_ts)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        (
            transaction["id"],
            int(transaction.get("amount_cents") or 0),
            transaction.get("memo") or "",
            transaction.get("date") or "",
            datetime.now(UTC).isoformat(),
            slack_ts,
        ),
    )
    conn.commit()


def fetch_hcb_fines() -> list[dict[str, Any]]:
    fines: list[dict[str, Any]] = []
    page = 1

    while True:
        response = requests.get(
            HCB_TRANSACTIONS_URL,
            params={"per_page": PAGE_SIZE, "page": page},
            timeout=TIMEOUT_SECONDS,
        )
        response.raise_for_status()
        transactions = response.json()
        if not isinstance(transactions, list):
            raise RuntimeError("Unexpected HCB response: expected a JSON array")
        if not transactions:
            break

        fines.extend(
            transaction
            for transaction in transactions
            if int(transaction.get("amount_cents") or 0) > 0
        )

        if len(transactions) < PAGE_SIZE:
            break
        page += 1

    return fines


def fetch_otter_fines(api_base: str) -> dict[str, dict[str, Any]]:
    fines: dict[str, dict[str, Any]] = {}
    page = 1

    while True:
        response = requests.get(
            f"{api_base.rstrip('/')}/api/v1/fines",
            params={"limit": PAGE_SIZE, "page": page},
            timeout=TIMEOUT_SECONDS,
        )
        response.raise_for_status()
        payload = response.json()

        if isinstance(payload, list):
            data = payload
            total = len(data)
            per_page = len(data) or PAGE_SIZE
        elif isinstance(payload, dict):
            data = payload.get("data") or []
            total = int(payload.get("total") or 0)
            per_page = int(payload.get("per_page") or PAGE_SIZE)
        else:
            raise RuntimeError("Unexpected Otter fines response")

        for fine in data:
            transaction_id = fine.get("transaction_id")
            if transaction_id:
                fines[str(transaction_id)] = fine

        if not data or page * per_page >= total:
            break
        page += 1

    return fines


def extract_ysws_from_memo(memo: str) -> str | None:
    memo = memo.strip()
    lower = memo.lower()
    if not lower.startswith("transfer from"):
        return None

    rest = memo[len("Transfer from") :].strip()
    for part in rest.replace("–", "-").split("-"):
        part = part.strip()
        if part and part.lower() not in {"ysws", "fines to ysws"}:
            return part
    return None


def amount_dollars(amount_cents: int | None) -> str:
    return f"${(amount_cents or 0) / 100:.2f}"


def dollars_value(amount_cents: int) -> str:
    return f"{amount_cents / 100:.2f}"


def timestamp_to_day(timestamp: int | None) -> str:
    if timestamp is None:
        return "unknown"
    return datetime.fromtimestamp(timestamp, tz=UTC).strftime("%Y-%m-%d")


def deleted_project_rows(fine: dict[str, Any] | None) -> list[dict[str, Any]]:
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
        for project in ((fine or {}).get("projects") or [])
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


def load_leaderboard_snapshot(conn: sqlite3.Connection) -> dict[str, int]:
    return {
        row[0]: int(row[1])
        for row in conn.execute(
            "SELECT ysws, total_cents FROM leaderboard_snapshot"
        ).fetchall()
    }


def load_last_leaderboard_posted_at(conn: sqlite3.Connection) -> int | None:
    row = conn.execute(
        "SELECT posted_at FROM leaderboard_state WHERE id = 1"
    ).fetchone()
    return int(row[0]) if row else None


def save_leaderboard_state(
    conn: sqlite3.Connection, leaderboard: dict[str, int], posted_at: int
) -> None:
    conn.execute("DELETE FROM leaderboard_snapshot")
    conn.executemany(
        "INSERT INTO leaderboard_snapshot (ysws, total_cents) VALUES (?, ?)",
        sorted(leaderboard.items()),
    )
    conn.execute(
        "INSERT OR REPLACE INTO leaderboard_state (id, posted_at) VALUES (1, ?)",
        (posted_at,),
    )
    conn.commit()


def build_leaderboard(
    hcb_fines: list[dict[str, Any]], otter_fines: dict[str, dict[str, Any]]
) -> dict[str, int]:
    leaderboard: dict[str, int] = {}
    for transaction in hcb_fines:
        transaction_id = str(transaction.get("id") or "")
        otter_fine = otter_fines.get(transaction_id) or {}
        ysws = (
            otter_fine.get("ysws")
            or extract_ysws_from_memo(transaction.get("memo") or "")
            or "unknown"
        )
        leaderboard[ysws] = leaderboard.get(ysws, 0) + int(
            transaction.get("amount_cents") or 0
        )
    return leaderboard


def leaderboard_rows(
    current: dict[str, int], previous: dict[str, int]
) -> list[dict[str, Any]]:
    rows = []
    for ysws in sorted(set(current) | set(previous)):
        total_cents = current.get(ysws, 0)
        change_cents = total_cents - previous.get(ysws, 0)
        rows.append(
            {
                "ysws": ysws,
                "total_dollars": dollars_value(total_cents),
                "change_dollars": dollars_value(change_cents),
                "_total_cents": total_cents,
                "_change_cents": change_cents,
            }
        )
    return sorted(rows, key=lambda row: int(row["_total_cents"]), reverse=True)


def write_leaderboard_csv(rows: list[dict[str, Any]]) -> Path:
    tmp = tempfile.NamedTemporaryFile(
        mode="w", newline="", encoding="utf-8", suffix=".csv", delete=False
    )
    with tmp:
        writer = csv.DictWriter(
            tmp, fieldnames=LEADERBOARD_FIELDNAMES, extrasaction="ignore"
        )
        writer.writeheader()
        writer.writerows(rows)
    return Path(tmp.name)


def leaderboard_comment(rows: list[dict[str, Any]]) -> str:
    if not rows:
        return "*Daily fines leaderboard*\nNo fines yet."

    changes = [row for row in rows if int(row["_change_cents"]) != 0]
    lines = ["*Daily fines leaderboard*"]

    for index, row in enumerate(rows[:10], start=1):
        change = int(row["_change_cents"])
        change_text = ""
        if change != 0:
            sign = "+" if change >= 0 else "-"
            change_text = f" ({sign}${dollars_value(abs(change))})"
        lines.append(f"{index}. *{row['ysws']}* - ${row['total_dollars']}{change_text}")

    if changes:
        lines.append("Changes:")
        for row in changes[:10]:
            change = int(row["_change_cents"])
            sign = "+" if change >= 0 else "-"
            lines.append(f"• {row['ysws']}: {sign}${dollars_value(abs(change))}")

    return "\n".join(lines)


def maybe_post_leaderboard(
    conn: sqlite3.Connection,
    client: WebClient,
    hcb_fines: list[dict[str, Any]],
    otter_fines: dict[str, dict[str, Any]],
    interval_seconds: int,
) -> None:
    now = int(time.time())
    previous = load_leaderboard_snapshot(conn)
    last_posted_at = load_last_leaderboard_posted_at(conn)
    current = build_leaderboard(hcb_fines, otter_fines)

    if last_posted_at is None:
        save_leaderboard_state(conn, current, now)
        print("Seeded leaderboard state; nothing posted.")
        return

    if now - last_posted_at < interval_seconds:
        return

    if current == previous:
        print("Leaderboard unchanged; nothing posted.")
        return

    rows = leaderboard_rows(current, previous)
    response = client.chat_postMessage(
        channel=FINES_CHANNEL_ID,
        text=leaderboard_comment(rows),
    )
    thread_ts = response.get("ts")

    csv_path = write_leaderboard_csv(rows)
    try:
        timestamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%SZ")
        client.files_upload_v2(
            channel=FINES_CHANNEL_ID,
            thread_ts=thread_ts,
            filename=f"fines-leaderboard-{timestamp}.csv",
            title="fines-leaderboard.csv",
            initial_comment="Full fines leaderboard CSV",
            file=str(csv_path),
        )
    finally:
        csv_path.unlink(missing_ok=True)

    save_leaderboard_state(conn, current, now)


def fine_comment(transaction: dict[str, Any], otter_fine: dict[str, Any] | None) -> str:
    projects = (otter_fine or {}).get("projects") or []
    amount_cents = int(transaction.get("amount_cents") or 0)
    ysws = (otter_fine or {}).get("ysws") or "unknown"
    transaction_id = transaction["id"]

    lines = [
        "*New fine*",
        f"Amount: {amount_dollars(amount_cents)}",
        f"YSWS: {ysws}",
        f"Date: {transaction.get('date') or (otter_fine or {}).get('date') or 'unknown'}",
    ]
    if projects:
        lines.append(f"Deleted projects: {len(projects)}")
    lines.append(f"<https://hcbscan.3kh0.net/app/txn/{transaction_id}|Transaction>")
    return "\n".join(lines)


def post_fine(
    client: WebClient, transaction: dict[str, Any], otter_fine: dict[str, Any] | None
) -> str | None:
    rows = deleted_project_rows(otter_fine)
    comment = fine_comment(transaction, otter_fine)

    if not rows:
        response = client.chat_postMessage(channel=FINES_CHANNEL_ID, text=comment)
        return response.get("ts")

    csv_path = write_csv(rows)
    timestamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%SZ")
    filename = f"deleted-projects-fine-{transaction['id']}-{timestamp}.csv"
    try:
        response = client.files_upload_v2(
            channel=FINES_CHANNEL_ID,
            filename=filename,
            title=filename,
            initial_comment=comment,
            file=str(csv_path),
        )
        return (response.get("file") or {}).get("shares", {}).get("ts")
    finally:
        csv_path.unlink(missing_ok=True)


def run_once(
    conn: sqlite3.Connection,
    client: WebClient,
    api_base: str,
    leaderboard_interval_seconds: int,
) -> int:
    already_sent = sent_ids(conn)
    hcb_fines = fetch_hcb_fines()
    otter_fines = fetch_otter_fines(api_base)

    maybe_post_leaderboard(
        conn, client, hcb_fines, otter_fines, leaderboard_interval_seconds
    )

    if not already_sent and hcb_fines:
        for transaction in hcb_fines:
            mark_sent(conn, transaction, None)
        print(f"Seeded {len(hcb_fines)} existing fine(s); nothing posted.")
        return 0

    pending = [fine for fine in hcb_fines if fine.get("id") not in already_sent]

    for transaction in sorted(
        pending, key=lambda tx: (tx.get("date") or "", tx.get("id") or "")
    ):
        otter_fine = otter_fines.get(str(transaction["id"]))
        slack_ts = post_fine(client, transaction, otter_fine)
        mark_sent(conn, transaction, slack_ts)

    print(f"Posted {len(pending)} fine(s).")
    return len(pending)


def main() -> int:
    env_file = Path(__file__).resolve().parent / ".env"
    root_env = Path(__file__).resolve().parents[1] / ".env"
    load_dotenv(dotenv_path=str(env_file if env_file.exists() else root_env))

    parser = argparse.ArgumentParser()
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args()

    client = WebClient(token=required_env("SLACK_BOT_TOKEN"), timeout=TIMEOUT_SECONDS)
    api_base = os.environ.get("OTTER_API_BASE", DEFAULT_API_BASE).rstrip("/")
    db_path = Path(os.environ.get("FINES_BOT_DB", DEFAULT_DB_PATH))
    interval = int(
        os.environ.get("FINES_BOT_INTERVAL_SECONDS", DEFAULT_INTERVAL_SECONDS)
    )
    leaderboard_interval = int(
        os.environ.get(
            "FINES_BOT_LEADERBOARD_INTERVAL_SECONDS",
            DEFAULT_LEADERBOARD_INTERVAL_SECONDS,
        )
    )

    with connect_db(db_path) as conn:
        while True:
            try:
                run_once(conn, client, api_base, leaderboard_interval)
            except Exception:  # pylint: disable=broad-except
                traceback.print_exc()
                if args.once:
                    return 1

            if args.once:
                return 0
            time.sleep(interval)


if __name__ == "__main__":
    raise SystemExit(main())
