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

DEFAULT_API_BASE = "http://localhost:3000"
DEFAULT_INTERVAL_SECONDS = 60 * 60
DEFAULT_LEADERBOARD_INTERVAL_SECONDS = 24 * 60 * 60
DEFAULT_DB_PATH = Path(__file__).resolve().parent / "fines-bot.sqlite3"
TIMEOUT_SECONDS = 20
PAGE_SIZE = 100
FINES_CHANNEL_ID = "C0B1X3W6MHS"
CHART_LABEL_MAX_LEN = 20
CHART_MAX_SEGMENTS = 12
FINE_ICON_URL = (
    "https://cdn.hackclub.com/019fcc78-cf47-7e97-a246-aa189e3547a9/1f7e5.png"
)
REVERTED_FINE_ICON_URL = (
    "https://cdn.hackclub.com/019fcc78-d708-7228-afda-ab5497cf97ad/1f7e9.png"
)

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
    conn: sqlite3.Connection, fine: dict[str, Any], slack_ts: str | None
) -> None:
    conn.execute(
        """
        INSERT OR REPLACE INTO sent_fines
        (transaction_id, amount_cents, memo, hcb_date, posted_at, slack_ts)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        (
            fine["transaction_id"],
            int(fine.get("amount_cents") or 0),
            fine.get("memo") or "",
            fine.get("date") or "",
            datetime.now(UTC).isoformat(),
            slack_ts,
        ),
    )
    conn.commit()


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
            raise TypeError("Unexpected Otter fines response")

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
    if rest.lower().startswith("fines to"):
        rest = rest[len("fines to") :].strip()

    for part in rest.replace("–", "-").split("-"):
        part = part.strip()
        if part and part.lower() != "ysws":
            return part
    return None


def amount_dollars(amount_cents: int | None) -> str:
    cents = amount_cents or 0
    sign = "-" if cents < 0 else ""
    return f"{sign}${abs(cents) / 100:.2f}"


def is_reverted_fine(transaction: dict[str, Any]) -> bool:
    return int(transaction.get("amount_cents") or 0) < 0


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


def fine_ysws(fine: dict[str, Any]) -> str:
    return (
        fine.get("ysws") or extract_ysws_from_memo(fine.get("memo") or "") or "unknown"
    )


def build_leaderboard(otter_fines: dict[str, dict[str, Any]]) -> dict[str, int]:
    leaderboard: dict[str, int] = {}
    for fine in otter_fines.values():
        ysws = fine_ysws(fine)
        leaderboard[ysws] = leaderboard.get(ysws, 0) + int(
            fine.get("amount_cents") or 0
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


def post_message(client: WebClient, **kwargs: Any) -> Any:
    """Post to Slack."""
    kwargs.setdefault("unfurl_links", False)
    kwargs.setdefault("unfurl_media", False)
    return client.chat_postMessage(**kwargs)


def chart_label(value: Any) -> str:
    return str(value)[:CHART_LABEL_MAX_LEN]


def table_cell(value: Any) -> dict[str, str]:
    text = str(value) if value not in (None, "") else "-"
    return {"type": "raw_text", "text": text}


def new_fines_chart_blocks(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    segments = [
        {
            "label": chart_label(row["ysws"]),
            "value": int(row["_change_cents"]) / 100,
        }
        for row in rows
        if int(row["_change_cents"]) > 0
    ][:CHART_MAX_SEGMENTS]

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
            "label": chart_label(row["ysws"]),
            "value": int(row["_total_cents"]) / 100,
        }
        for row in rows
        if int(row["_total_cents"]) > 0
    ][:CHART_MAX_SEGMENTS]

    if not data:
        return []

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
    table_rows = [[table_cell(fieldname) for fieldname in LEADERBOARD_FIELDNAMES]]

    for row in rows:
        table_rows.append(
            [table_cell(row.get(fieldname)) for fieldname in LEADERBOARD_FIELDNAMES]
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
    otter_fines: dict[str, dict[str, Any]],
    interval_seconds: int,
) -> None:
    now = int(time.time())
    previous = load_leaderboard_snapshot(conn)
    last_posted_at = load_last_leaderboard_posted_at(conn)
    current = build_leaderboard(otter_fines)

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
        post_message(
            client,
            channel=FINES_CHANNEL_ID,
            text="New fines",
            blocks=new_fines_blocks,
        )

    leaderboard_blocks = leaderboard_chart_blocks(rows)
    if leaderboard_blocks:
        response = post_message(
            client,
            channel=FINES_CHANNEL_ID,
            text="Fines leaderboard",
            blocks=leaderboard_blocks,
        )
        leaderboard_thread_ts = response.get("ts")

        post_message(
            client,
            channel=FINES_CHANNEL_ID,
            thread_ts=leaderboard_thread_ts,
            text="Full fines leaderboard",
            blocks=leaderboard_table_blocks(rows),
        )

    save_leaderboard_state(conn, current, now)


def fine_comment(fine: dict[str, Any]) -> str:
    reverted = is_reverted_fine(fine)
    projects = [] if reverted else fine.get("projects") or []
    amount_cents = int(fine.get("amount_cents") or 0)
    ysws = fine_ysws(fine)
    transaction_id = fine["transaction_id"]

    lines = [
        "*Fine reverted*" if reverted else "*New fine*",
        f"Amount: {amount_dollars(amount_cents)}",
        f"YSWS: {ysws}",
        f"Date: {fine.get('date') or 'unknown'}",
    ]
    if projects:
        lines.append(f"Deleted projects: {len(projects)}")
    lines.append(f"<https://hcbscan.3kh0.net/app/txn/{transaction_id}|Transaction>")
    return "\n".join(lines)


def fine_blocks(fine: dict[str, Any]) -> list[dict[str, Any]]:
    reverted = is_reverted_fine(fine)
    amount_cents = int(fine.get("amount_cents") or 0)
    ysws = fine_ysws(fine)
    projects = [] if reverted else fine.get("projects") or []
    date = fine.get("date") or "unknown"
    transaction_id = fine["transaction_id"]
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
            "icon": {
                "type": "image",
                "image_url": REVERTED_FINE_ICON_URL if reverted else FINE_ICON_URL,
                "alt_text": "Reverted Fine" if reverted else "Fine",
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
    table_rows = [[table_cell(fieldname) for fieldname in CSV_FIELDNAMES]]

    for row in rows:
        table_rows.append(
            [table_cell(row.get(fieldname)) for fieldname in CSV_FIELDNAMES]
        )

    return [
        {
            "type": "data_table",
            "caption": "Deleted Projects",
            "rows": table_rows,
        }
    ]


def post_fine(client: WebClient, fine: dict[str, Any]) -> str | None:
    rows = [] if is_reverted_fine(fine) else deleted_project_rows(fine)
    comment = fine_comment(fine)

    response = post_message(
        client,
        channel=FINES_CHANNEL_ID,
        text=comment,
        blocks=fine_blocks(fine),
    )
    thread_ts = response.get("ts")

    if not rows:
        return thread_ts

    post_message(
        client,
        channel=FINES_CHANNEL_ID,
        thread_ts=thread_ts,
        text="Deleted projects",
        blocks=deleted_projects_table_blocks(rows),
    )
    return thread_ts


def run_once(
    conn: sqlite3.Connection,
    client: WebClient,
    api_base: str,
    leaderboard_interval_seconds: int,
) -> int:
    already_sent = sent_ids(conn)
    otter_fines = fetch_otter_fines(api_base)

    if not otter_fines:
        print("Otter API returned no fines; skipping this run.")
        return 0

    maybe_post_leaderboard(conn, client, otter_fines, leaderboard_interval_seconds)

    if not already_sent and otter_fines:
        for fine in otter_fines.values():
            mark_sent(conn, fine, None)
        print(f"Seeded {len(otter_fines)} existing fine(s); nothing posted.")
        return 0

    pending = [
        fine
        for fine in otter_fines.values()
        if fine.get("transaction_id") not in already_sent
    ]

    for fine in sorted(
        pending, key=lambda f: (f.get("date") or "", f.get("transaction_id") or "")
    ):
        slack_ts = post_fine(client, fine)
        mark_sent(conn, fine, slack_ts)

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
            except Exception:  # pylint: disable=broad-except  # noqa: BLE001
                traceback.print_exc()
                if args.once:
                    return 1

            if args.once:
                return 0
            time.sleep(interval)


if __name__ == "__main__":
    raise SystemExit(main())
