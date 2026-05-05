"""Otter Slack bot"""

import os
import re
import threading
import time
import traceback
from typing import Optional, TypedDict

import fines_notifier
import requests
from dotenv import load_dotenv
from slack_bolt import App
from slack_bolt.adapter.socket_mode import SocketModeHandler
from slack_sdk.errors import SlackApiError

FINES_NOTIFIER_INTERVAL_SECONDS = 60 * 60


class ProjectItem(TypedDict, total=False):
    """Type for project items received from the API"""

    id: int
    airtable_id: str
    approved_at: Optional[int]
    display_name: Optional[str]
    description: Optional[str]
    ysws: str
    country: Optional[str]
    code_url: Optional[str]
    demo_url: Optional[str]
    github_username: Optional[str]
    hours: Optional[int]
    true_hours: Optional[float]
    has_media: bool
    github_stars: int
    archived_demo: Optional[str]
    archived_repo: Optional[str]
    inferred_repo: Optional[str]
    inferred_username: Optional[str]
    preview_blurhash: Optional[str]


load_dotenv()

app = App(token=os.environ["SLACK_BOT_TOKEN"])

API_BASE = os.environ.get("OTTER_API_BASE", "http://localhost:3000").rstrip("/")
FRONTEND_BASE = os.environ.get("OTTER_FRONTEND_BASE", "http://localhost:5173").rstrip(
    "/"
)
OTTER_OWNER_ID = os.environ.get("OTTER_OWNER_ID", "").strip()

USER_MENTION_RE = re.compile(r"<@([UW][A-Z0-9]+)(?:\|[^>]+)?>")
GITHUB_URL_RE = re.compile(
    r"(?:https?://)?(?:www\.)?github\.com/([A-Za-z0-9-]+)", re.IGNORECASE
)
GITHUB_USERNAME_RE = re.compile(r"^[A-Za-z0-9-]{1,39}$")

GITHUB_FIELD_ID = "Xf09V176UVK5"

OTTER_CHANNEL_ID = "C0B0603KY6T"


def media_image_url(airtable_id: str | None) -> str | None:
    """Get the media url from an airtable id"""
    if not airtable_id:
        return None
    return f"{API_BASE}/api/v1/media/{airtable_id}/r"


def pluralize(count: int, singular: str, plural: Optional[str] = None) -> str:
    """Return a pluralized string based on the count"""
    if count == 1:
        return singular

    return plural or singular + "s"


def fetch_projects_for_user(github_username: str) -> list[ProjectItem]:
    """Fetch projects for a given username"""

    url = f"{API_BASE}/api/v1/search?limit=100&page=1&q=user:{github_username}"
    resp = requests.get(url, timeout=10)
    resp.raise_for_status()

    data = resp.json().get("data", [])

    return data


def parse_target(text: str, sender_id: str):
    """Return the requested target as either a Slack user id or GitHub username"""
    text = (text or "").strip()
    if not text or text.lower() in {"me", "my", "mine"}:
        return {"kind": "slack", "value": sender_id, "label": f"<@{sender_id}>"}

    match = USER_MENTION_RE.search(text)
    if match:
        user_id = match.group(1)
        return {"kind": "slack", "value": user_id, "label": f"<@{user_id}>"}

    github_match = GITHUB_URL_RE.search(text)
    if github_match:
        github_username = github_match.group(1)
        return {
            "kind": "github",
            "value": github_username,
            "label": f"GitHub user {github_username}",
        }

    token = text.split()[0].lstrip("@").rstrip("/")

    if GITHUB_USERNAME_RE.fullmatch(token) and token[0].isalnum():
        return {"kind": "github", "value": token, "label": f"GitHub user {token}"}

    if re.fullmatch(r"[UW][A-Z0-9]+", token):
        return {"kind": "slack", "value": token, "label": f"<@{token}>"}

    return {"kind": "slack", "value": sender_id, "label": f"<@{sender_id}>"}


def normalize_description(
    text: str | None, max_lines: int = 2, max_width: int = 35
) -> str:
    """Format description"""
    if not text:
        return "\n" * (max_lines - 1)

    text = text.strip()
    lines = []
    current_line = ""
    words = text.split()
    word_idx = 0

    for word_idx, word in enumerate(words):
        test_line = (current_line + " " if current_line else "") + word
        if len(test_line) <= max_width:
            current_line = test_line
        else:
            if current_line:
                lines.append(current_line)
            current_line = word
            if len(lines) >= max_lines:
                break

    if current_line and len(lines) < max_lines:
        lines.append(current_line)

    while len(lines) < max_lines:
        lines.append("")

    is_truncated = word_idx < len(words) - 1
    if is_truncated and lines:
        lines[-1] = (lines[-1].rstrip() + "…")[-max_width:]

    return "\n".join(lines[:max_lines])


def deduplicate_projects(projects: list[ProjectItem]) -> list[ProjectItem]:
    """Keep only the most recent version of each project by name"""
    seen = {}
    for project in projects:
        repo_name = project.get("inferred_repo", "")
        if not repo_name:
            continue

        if repo_name not in seen or project.get("id", 0) > seen[repo_name].get("id", 0):
            seen[repo_name] = project

    return list(seen.values())


def log_command(command, success: bool):
    """Log command usage"""
    failed_text = "FAILED - " if not success else ""
    print(
        f"{failed_text}#{command['channel_name']} ({command['team_domain']}) - "
        f"{command['user_name']} ({command['user_id']}) "
        f"ran {command['command']} {command['text']}"
    )


def send_projects_response(
    *,
    client,
    channel_id: str,
    source_user_id: str,
    text: str,
    thread_ts: str | None = None,
):
    """Build and send the projects response."""
    github_username = None
    user = {}
    profile = {}

    target = parse_target(text, source_user_id)

    if target["kind"] == "slack":
        target_user_id = target["value"]

        try:
            info = client.users_info(user=target_user_id)
            profile_resp = client.users_profile_get(user=target_user_id)
        except Exception as exc:  # pylint: disable=broad-except
            return {
                "ok": False,
                "error_code": "user_lookup_failed",
                "error": f":warning: Could not fetch user `{target_user_id}`: {exc}",
            }

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
    projects = sorted(projects, key=lambda p: p.get("github_stars", 0), reverse=True)
    projects = deduplicate_projects(projects)

    requested_by = (
        f"(requested by <@{source_user_id}>)"
        if source_user_id != target.get("value")
        else ""
    )

    if target["kind"] == "github" and username:
        target_text = f"<https://github.com/{username}|{username}>"
    elif target["kind"] == "slack":
        target_text = f"<@{target['value']}>"
    else:
        target_text = target["label"]

    header_text = f"{target_text}*'s projects* {requested_by}"

    blocks = [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": header_text,
            },
        }
    ]

    if target["kind"] == "slack" and not github_username and username:
        blocks.append(
            {
                "type": "context",
                "elements": [
                    {
                        "type": "mrkdwn",
                        "text": ":siren1: This user hasn't set their GitHub "
                        "in their profile. Results may be inaccurate. :siren1:",
                    }
                ],
            }
        )

    if projects:
        carousel_cards = []
        for idx, project in enumerate(projects[:10]):
            airtable_id = project.get("airtable_id")
            repo_name = project.get("inferred_repo", "Unknown")
            description = project.get("description", "")
            stars = project.get("github_stars", 0)
            hours = project.get("true_hours") or project.get("hours", 0)
            ysws = project.get("ysws")
            image_url = media_image_url(airtable_id)

            subtitle_parts = []
            if ysws:
                subtitle_parts.append(ysws)
            if stars:
                subtitle_parts.append(f":star: {stars} stars")
            if hours:
                subtitle_parts.append(f":clock2: {round(hours, 1)}h")
            subtitle = " • ".join(subtitle_parts) if subtitle_parts else "No stats"

            actions = []
            if airtable_id:
                actions.append(
                    {
                        "type": "button",
                        "text": {
                            "type": "plain_text",
                            "text": "Open",
                            "emoji": True,
                        },
                        "url": f"{FRONTEND_BASE}/project/{airtable_id}",
                        "action_id": f"id_{project.get('airtable_id', 'unknown')}",
                    }
                )

            card = {
                "type": "card",
                "block_id": f"project-card-{idx}",
                **(
                    {
                        "hero_image": {
                            "type": "image",
                            "image_url": image_url,
                            "alt_text": f"Preview image for {repo_name}",
                        }
                    }
                    if image_url
                    else {}
                ),
                "title": {
                    "type": "mrkdwn",
                    "text": f"*{repo_name}*",
                    "verbatim": False,
                },
                "subtitle": {
                    "type": "mrkdwn",
                    "text": subtitle,
                    "verbatim": False,
                },
                "body": {
                    "type": "mrkdwn",
                    "text": normalize_description(description)
                    or "_No description available_",
                    "verbatim": False,
                },
            }

            if actions:
                card["actions"] = actions

            carousel_cards.append(card)

        blocks.append(
            {
                "type": "carousel",
                "elements": carousel_cards,
            }
        )

        count = len(projects)
        if count > 10:
            blocks.append(
                {
                    "type": "context",
                    "elements": [
                        {
                            "type": "mrkdwn",
                            "text": f"_and {count - 10} more {pluralize(count - 10, 'project')}"
                            " (showing 10)_",
                        }
                    ],
                }
            )
    else:
        blocks.append(
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "_No projects found_",
                },
            }
        )

    if github_username:
        blocks.append(
            {
                "type": "context",
                "elements": [
                    {
                        "type": "mrkdwn",
                        "text": f"<{FRONTEND_BASE}/?q=user%3A{github_username}|View all projects>",
                    }
                ],
            }
        )

    try:
        client.chat_postMessage(
            channel=channel_id,
            thread_ts=thread_ts,
            blocks=blocks,
            text=f"{target['label']}'s projects {requested_by}",
            unfurl_links=False,
            icon_emoji=":otter:",
            metadata={
                "event_type": "otter_message",
                "event_payload": {"source_user_id": source_user_id},
            },
        )
    except SlackApiError as exc:
        return {
            "ok": False,
            "error_code": exc.response.get("error"),
            "error": str(exc),
        }

    return {"ok": True}


@app.action(re.compile(r"^id_.*"))
def handle_project_link(ack, body):
    """Log project link clicks"""
    ack()
    action_id = body["actions"][0]["action_id"]
    user_id = body["user"]["id"]
    username = body["user"]["username"]
    print(f"click: {action_id[3:]} by {username} ({user_id})")


@app.shortcut("delete_otter_message")
def handle_delete_bot_message(ack, body, client):
    """Delete an Otter bot message"""
    ack()

    clicked_user_id = body["user"]["id"]
    message = body.get("message", {}) or {}
    metadata = message.get("metadata", {}) or {}
    event_payload = metadata.get("event_payload", {}) or {}
    requester_user_id = event_payload.get("source_user_id", "")
    authorized_user_ids = {requester_user_id}

    if OTTER_OWNER_ID:
        authorized_user_ids.add(OTTER_OWNER_ID)

    if clicked_user_id not in authorized_user_ids:
        channel_id = body.get("container", {}).get("channel_id") or body.get(
            "channel", {}
        ).get("id")
        if channel_id:
            client.chat_postEphemeral(
                channel=channel_id,
                user=clicked_user_id,
                text="You are not allowed to delete this message.",
            )
        return

    if not (message.get("bot_id") or message.get("subtype") == "bot_message"):
        print("delete_bot_message: refusing to delete a non-bot message")
        return

    container = body.get("container", {}) or {}
    channel_id = container.get("channel_id") or body.get("channel", {}).get("id")
    message_ts = container.get("message_ts") or message.get("ts")

    if not channel_id or not message_ts:
        print("delete_bot_message: missing channel or message timestamp")
        return

    try:
        client.chat_delete(channel=channel_id, ts=message_ts)
    except SlackApiError as exc:
        print(f"delete_bot_message failed: {exc.response.get('error')}")


@app.command("/otter")
def list_projects(ack, command, client, respond):
    """Handle the /otter command"""
    ack()

    result = send_projects_response(
        client=client,
        channel_id=command["channel_id"],
        source_user_id=command["user_id"],
        text=command.get("text", ""),
    )

    if result["ok"]:
        log_command(command, success=True)
        return

    error_code = result.get("error_code")
    error_text = result.get("error")

    if error_code == "user_lookup_failed" and isinstance(error_text, str):
        respond(error_text)
        return

    if error_code == "channel_not_found":
        log_command(command, success=False)
        respond("Please add me to the channel before using the command!")
        return

    log_command(command, success=False)
    print(f"Error handling /otter command -> {error_code}: {error_text}")
    respond(f"{error_code}: {error_text}")
    return


@app.event("message")
def handle_message_events(body, client):
    """Send projects in thread"""
    event = body.get("event", {})
    channel_id = event.get("channel")
    user_id = event.get("user")
    text = event.get("text", "")

    if (
        channel_id == OTTER_CHANNEL_ID
        and user_id
        and not event.get("bot_id")
        and not event.get("subtype")
        and not event.get("thread_ts")
    ):
        result = send_projects_response(
            client=client,
            channel_id=channel_id,
            source_user_id=user_id,
            text=text,
            thread_ts=event.get("thread_ts") or event.get("ts"),
        )

        if not result["ok"]:
            print(result["error"])


def fines_notifier_loop():
    """Run the fine notifier every hour"""
    while True:
        try:
            fines_notifier.main()
        except Exception:  # pylint: disable=broad-except
            traceback.print_exc()
        time.sleep(FINES_NOTIFIER_INTERVAL_SECONDS)


def main():
    """Main entry point"""

    threading.Thread(target=fines_notifier_loop, daemon=True).start()

    handler = SocketModeHandler(app, os.environ["SLACK_APP_TOKEN"])
    handler.start()


if __name__ == "__main__":
    main()
