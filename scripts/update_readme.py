#!/usr/bin/env python3
"""Refresh the auto-generated build info section in README.md."""

from __future__ import annotations

import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
CHANGELOG = ROOT / "CHANGELOG.md"
CARGO_TOML = ROOT / "Cargo.toml"

START_MARKER = "<!-- readme:build-info:start -->"
END_MARKER = "<!-- readme:build-info:end -->"
REPO = "itsinvin/QuartzLauncher"


def read_version() -> str:
    if env_version := __import__("os").environ.get("PANDORA_RELEASE_VERSION"):
        return env_version.lstrip("v")

    text = CARGO_TOML.read_text(encoding="utf-8")
    match = re.search(r'^\s*version\s*=\s*"([^"]+)"\s*$', text, re.MULTILINE)
    if not match:
        raise SystemExit("Could not read workspace version from Cargo.toml")
    return match.group(1)


def git_short_sha() -> str | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout.strip() or None
    except (OSError, subprocess.CalledProcessError):
        return None


def parse_changelog_section(version: str) -> tuple[str | None, list[str]]:
    if not CHANGELOG.exists():
        return None, []

    lines = CHANGELOG.read_text(encoding="utf-8").splitlines()
    header_prefix = f"## [{version}]"
    start = next(
        (i for i, line in enumerate(lines) if line.strip().startswith(header_prefix)),
        None,
    )
    if start is None:
        return None, []

    date = None
    header_match = re.match(r"^## \[([^\]]+)\](?: - (\d{4}-\d{2}-\d{2}))?$", lines[start].strip())
    if header_match:
        date = header_match.group(2)

    bullets: list[str] = []
    for line in lines[start + 1 :]:
        if line.startswith("## ["):
            break
        stripped = line.strip()
        if stripped.startswith("- "):
            bullets.append(stripped[2:])

    return date, bullets[:8]


def render_section(version: str) -> str:
    built_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    sha = git_short_sha()
    changelog_date, bullets = parse_changelog_section(version)
    release_url = f"https://github.com/{REPO}/releases/tag/v{version}"

    rows = [
        f"| **Version** | `{version}` |",
        f"| **Last built** | {built_at} |",
        f"| **Latest release** | [v{version}]({release_url}) |",
    ]
    if sha:
        rows.append(f"| **Commit** | `{sha}` |")
    if changelog_date:
        rows.append(f"| **Changelog date** | {changelog_date} |")

    body = [
        START_MARKER,
        "## Build info",
        "",
        "This section is updated automatically after every build.",
        "",
        "| | |",
        "|---|---|",
        *rows,
    ]

    if bullets:
        body.extend(["", f"### Recent changes ({version})", ""])
        body.extend(f"- {bullet}" for bullet in bullets)

    body.extend(["", END_MARKER])
    return "\n".join(body) + "\n"


def update_readme(version: str | None = None) -> bool:
    version = version or read_version()
    section = render_section(version)

    if not README.exists():
        raise SystemExit(f"Missing {README}")

    text = README.read_text(encoding="utf-8")
    pattern = re.compile(
        re.escape(START_MARKER) + r".*?" + re.escape(END_MARKER) + r"\n?",
        re.DOTALL,
    )

    if not pattern.search(text):
        raise SystemExit(
            f"README.md is missing build info markers:\n{START_MARKER}\n{END_MARKER}"
        )

    updated = pattern.sub(section, text, count=1)
    if updated == text:
        return False

    README.write_text(updated, encoding="utf-8", newline="\n")
    return True


def main() -> None:
    version = sys.argv[1].lstrip("v") if len(sys.argv) > 1 else None
    changed = update_readme(version)
    print(f"README {'updated' if changed else 'already up to date'} (v{version or read_version()})")


if __name__ == "__main__":
    main()
