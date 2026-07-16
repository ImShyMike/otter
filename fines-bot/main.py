"""Posts YSWS fines to Slack"""

from __future__ import annotations

import argparse
import os
import sqlite3
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
    "total",
    "change",
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
                "total": dollars_value(total_cents),
                "change": dollars_value(change_cents),
                "_total_cents": total_cents,
                "_change_cents": change_cents,
            }
        )
    return sorted(rows, key=lambda row: int(row["_total_cents"]), reverse=True)


def new_fines_chart_blocks(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    segments = [
        {
            "label": str(row["ysws"]),
            "value": int(row["_change_cents"]) / 100,
        }
        for row in rows
        if int(row["_change_cents"]) > 0
    ]

    if not segments:
        return []

    return [
        {
            "type": "data_visualization",
            "block_id": "viz-pie-new-fines",
            "title": "New Fines",
            "chart": {
                "type": "pie",
                "segments": segments,
            },
        }
    ]


def leaderboard_chart_blocks(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    data = [
        {
            "label": str(row["ysws"]),
            "value": int(row["_total_cents"]) / 100,
        }
        for row in rows
        if int(row["_total_cents"]) > 0
    ]

    return [
        {
            "type": "data_visualization",
            "block_id": "viz-bar-fines-leaderboard",
            "title": "Fines Leaderboard",
            "chart": {
                "type": "bar",
                "series": [
                    {
                        "name": "Dollars",
                        "data": data,
                    }
                ],
                "axis_config": {
                    "categories": [row["label"] for row in data],
                    "x_label": "YSWS",
                    "y_label": "Dollars",
                },
            },
        }
    ]


def leaderboard_table_blocks(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    table_rows = [
        [
            {
                "type": "raw_text",
                "text": fieldname,
            }
            for fieldname in LEADERBOARD_FIELDNAMES
        ]
    ]

    for row in rows:
        table_rows.append(
            [
                {
                    "type": "raw_text",
                    "text": str(row.get(fieldname) or ""),
                }
                for fieldname in LEADERBOARD_FIELDNAMES
            ]
        )

    return [
        {
            "type": "data_table",
            "caption": "Fines Leaderboard",
            "rows": table_rows,
        }
    ]


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
    new_fines_blocks = new_fines_chart_blocks(rows)
    if new_fines_blocks:
        client.chat_postMessage(
            channel=FINES_CHANNEL_ID,
            text="New fines",
            blocks=new_fines_blocks,
        )

    response = client.chat_postMessage(
        channel=FINES_CHANNEL_ID,
        text="Fines leaderboard",
        blocks=leaderboard_chart_blocks(rows),
    )
    leaderboard_thread_ts = response.get("ts")

    client.chat_postMessage(
        channel=FINES_CHANNEL_ID,
        thread_ts=leaderboard_thread_ts,
        text="Full fines leaderboard",
        blocks=leaderboard_table_blocks(rows),
    )

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


def fine_blocks(
    transaction: dict[str, Any], otter_fine: dict[str, Any] | None
) -> list[dict[str, Any]]:
    amount_cents = int(transaction.get("amount_cents") or 0)
    ysws = (otter_fine or {}).get("ysws") or "unknown"
    projects = (otter_fine or {}).get("projects") or []
    date = transaction.get("date") or (otter_fine or {}).get("date") or "unknown"
    transaction_id = transaction["id"]
    transaction_link = (
        f"<https://hcbscan.3kh0.net/app/txn/{transaction_id}|Transaction>"
    )
    context_text = transaction_link

    if projects:
        project_word = "project" if len(projects) == 1 else "projects"
        context_text = f"{len(projects)} {project_word} deleted\n\n{transaction_link}"

    return [
        {
            "type": "container",
            "block_id": f"fine_{transaction_id}",
            "title": {
                "type": "plain_text",
                "text": ysws,
            },
            "subtitle": {
                "type": "plain_text",
                "text": date,
            },
            "has_header_divider": True,
            "child_blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": amount_dollars(amount_cents),
                        "emoji": True,
                    },
                    "level": 1,
                },
                {
                    "type": "context",
                    "block_id": "transaction_context",
                    "elements": [
                        {
                            "type": "mrkdwn",
                            "text": context_text,
                        }
                    ],
                },
            ],
        }
    ]


def deleted_projects_table_blocks(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    table_rows = [
        [
            {
                "type": "raw_text",
                "text": fieldname,
            }
            for fieldname in CSV_FIELDNAMES
        ]
    ]

    for row in rows:
        table_rows.append(
            [
                {
                    "type": "raw_text",
                    "text": str(row.get(fieldname) or ""),
                }
                for fieldname in CSV_FIELDNAMES
            ]
        )

    return [
        {
            "type": "data_table",
            "caption": "Deleted Projects",
            "rows": table_rows,
        }
    ]


def post_fine(
    client: WebClient, transaction: dict[str, Any], otter_fine: dict[str, Any] | None
) -> str | None:
    rows = deleted_project_rows(otter_fine)
    comment = fine_comment(transaction, otter_fine)

    response = client.chat_postMessage(
        channel=FINES_CHANNEL_ID,
        text=comment,
        blocks=fine_blocks(transaction, otter_fine),
        unfurl_links=False,
    )
    thread_ts = response.get("ts")

    if not rows:
        return thread_ts

    client.chat_postMessage(
        channel=FINES_CHANNEL_ID,
        thread_ts=thread_ts,
        text="Deleted projects",
        blocks=deleted_projects_table_blocks(rows),
        unfurl_links=False,
    )
    return thread_ts


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
