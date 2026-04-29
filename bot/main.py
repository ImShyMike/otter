"""Otter Slack bot"""

import json
import os
import re

import requests
from dotenv import load_dotenv
from slack_bolt import App
from slack_bolt.adapter.socket_mode import SocketModeHandler

load_dotenv()

app = App(token=os.environ["SLACK_BOT_TOKEN"])

API_BASE = os.environ.get("OTTER_API_BASE", "http://localhost:3000").rstrip("/")

USER_MENTION_RE = re.compile(r"<@([UW][A-Z0-9]+)(?:\|[^>]+)?>")
GITHUB_URL_RE = re.compile(
    r"(?:https?://)?(?:www\.)?github\.com/([A-Za-z0-9-]+)", re.IGNORECASE
)
GITHUB_USERNAME_RE = re.compile(r"^[A-Za-z0-9-]{1,39}$")

GITHUB_FIELD_ID = "Xf09V176UVK5"


def fetch_projects_for_user(github_username: str):
    """Fetch a list of projects for a given username"""

    url = f"{API_BASE}/api/v1/search?limit=100&page=1&q=user:{github_username}"
    resp = requests.get(url, timeout=10)
    resp.raise_for_status()

    data = resp.json().get("data", [])

    return data


def parse_target(text: str, sender_id: str):
    """Return the requested target as either a Slack user id or GitHub username"""
    text = (text or "").strip()
    if not text:
        return {"kind": "slack_user", "value": sender_id, "label": f"<@{sender_id}>"}

    match = USER_MENTION_RE.search(text)
    if match:
        user_id = match.group(1)
        return {"kind": "slack_user", "value": user_id, "label": f"<@{user_id}>"}

    github_match = GITHUB_URL_RE.search(text)
    if github_match:
        github_username = github_match.group(1)
        return {
            "kind": "github_user",
            "value": github_username,
            "label": f"GitHub user {github_username}",
        }

    token = text.split()[0].lstrip("@").rstrip("/")

    if GITHUB_USERNAME_RE.fullmatch(token) and token[0].isalnum():
        return {"kind": "github_user", "value": token, "label": f"GitHub user {token}"}

    if re.fullmatch(r"[UW][A-Z0-9]+", token):
        return {"kind": "slack_user", "value": token, "label": f"<@{token}>"}

    return {"kind": "slack_user", "value": sender_id, "label": f"<@{sender_id}>"}


@app.command("/otter")
def list_projects(ack, command, client, respond):
    """Handle the /otter command"""
    ack()

    github_username = None
    user = {}
    profile = {}

    target = parse_target(command.get("text", ""), command["user_id"])

    if target["kind"] == "slack_user":
        target_user_id = target["value"]

        try:
            info = client.users_info(user=target_user_id)
            profile_resp = client.users_profile_get(user=target_user_id)
        except Exception as exc:  # pylint: disable=broad-except
            respond(f":warning: Could not fetch user `{target_user_id}`: {exc}")
            return

        user = info.get("user", {}) or {}
        profile = {
            **(user.get("profile", {}) or {}),
            **(profile_resp.get("profile", {}) or {}),
        }

        github_username_field = profile.get("fields", {}).get(GITHUB_FIELD_ID)
        if github_username_field:
            github_url = github_username_field.get("value")
            if github_url:
                match = GITHUB_URL_RE.search(github_url)
                if match:
                    github_username = profile["github_username"] = match.group(1)
    else:
        github_username = target["value"]
        profile["github_username"] = github_username

    username = (
        github_username or profile.get("display_name") or profile.get("real_name")
    )
    projects = fetch_projects_for_user(username) if username else []
    project_names = [proj["inferred_repo"] for proj in projects]

    data = {
        "id": user.get("id"),
        "display_name": profile.get("display_name"),
        "real_name": profile.get("real_name"),
        "github_username": github_username,
        "projects": project_names,
    }
    pretty = json.dumps(data, indent=2, ensure_ascii=False)
    if len(pretty) > 2900:
        pretty = pretty[:2900] + "\n... (truncated)"

    respond(
        {
            "response_type": "ephemeral",
            "text": f"Profile for {target['label']}:\n```{pretty}```",
        }
    )


def main():
    """Main entry point"""

    handler = SocketModeHandler(app, os.environ["SLACK_APP_TOKEN"])
    handler.start()


if __name__ == "__main__":
    main()
