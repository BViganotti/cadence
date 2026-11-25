#!/usr/bin/env python3
"""Playwright acceptance test for the Cadence Studio.

Run with:

    python3 tests/browser/studio_playwright.py

Requires the Python Playwright package and a Chromium browser. The Rust
workspace itself has no Python or Playwright dependency; this is a development
test only, documented in docs/testing.md.
"""

from __future__ import annotations

import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

from playwright.sync_api import sync_playwright


ROOT = Path(__file__).resolve().parents[2]


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_health(port: int, timeout: float = 15.0) -> None:
    deadline = time.time() + timeout
    url = f"http://127.0.0.1:{port}/healthz"
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=0.5) as response:
                if response.status == 200:
                    return
        except Exception:
            time.sleep(0.1)
    raise RuntimeError(f"studio did not become healthy on port {port}")


def main() -> int:
    binary = Path(os.environ.get("CADENCE_BIN", ROOT / "target/debug/cadence"))
    if not binary.exists():
        print(f"missing binary {binary}; run cargo build -p cadence", file=sys.stderr)
        return 2
    port = free_port()
    token = "playwright-token-123456"
    env = os.environ.copy()
    env.update(
        {
            "CADENCE_PORT": str(port),
            "CADENCE_TOKEN": token,
            "CADENCE_NO_OPEN": "1",
        }
    )
    server = subprocess.Popen(
        [str(binary)],
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        wait_for_health(port)
        url = f"http://127.0.0.1:{port}/?token={token}"
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch()
            page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
            console_errors: list[str] = []
            page.on(
                "console",
                lambda message: console_errors.append(f"{message.type}: {message.text}")
                if message.type in ("error", "warning")
                else None,
            )
            page.on("pageerror", lambda error: console_errors.append(f"pageerror: {error}"))

            page.goto(url, wait_until="domcontentloaded")
            page.wait_for_selector("#scoreStage svg", timeout=20000)
            page.wait_for_timeout(400)

            assert "Cadence" in page.title(), page.title()
            assert page.locator("#scoreTitle").inner_text() == "Cadence in G"
            assert page.locator("#parts .part").count() >= 3  # full score + 2 parts
            assert "no diagnostics" in page.locator("#diagnosticBadge").inner_text()
            assert page.locator("#scoreStage .hit").count() > 0
            assert page.url.endswith("/"), "access token should be removed from the URL"

            # Full-score preview includes both staves.
            page.locator("#parts .part", has_text="Full score").click()
            page.wait_for_timeout(600)
            full_hits = page.locator("#scoreStage .hit").count()
            assert full_hits > 20, full_hits

            # Select the bass staff and confirm hit coordinates map back to it.
            page.locator("#parts .part", has_text="Bass").click()
            page.wait_for_timeout(500)
            page.locator("#scoreStage .hit").first.click()
            page.wait_for_timeout(400)
            selection = page.locator("#selectionInfo").inner_text()
            assert "half" in selection, selection
            assert "measure 1" in selection, selection
            assert "G3" in selection, selection

            # Edit: transpose the selected note by changing its pitch.
            page.locator("#inspPitch").fill("A4")
            page.locator("#applyPitch").click()
            page.wait_for_timeout(500)
            assert "A4" in page.locator("#selectionInfo").inner_text()

            # Preview responds to a measure addition and note entry.
            page.locator("#parts .part", has_text="Melody").click()
            page.wait_for_timeout(400)
            before_notes = int(page.locator("#scoreMeta").inner_text().split(" notes")[0].split("·")[-1].strip())
            page.locator("#addMeasure").click()
            page.wait_for_timeout(500)
            assert page.locator("#measureCount").inner_text() == "9"
            page.locator("#measures .chip").last.click()
            page.locator("#durations .chip", has_text="quarter").click()
            page.keyboard.press("c")
            page.wait_for_timeout(600)
            after_notes = int(page.locator("#scoreMeta").inner_text().split(" notes")[0].split("·")[-1].strip())
            assert after_notes == before_notes + 1, (before_notes, after_notes)
            # Keyboard undo restores the previous score, then re-add the note.
            page.keyboard.press("Meta+z")
            page.wait_for_timeout(600)
            undone_notes = int(page.locator("#scoreMeta").inner_text().split(" notes")[0].split("·")[-1].strip())
            assert undone_notes == before_notes, (before_notes, undone_notes)
            page.keyboard.press("c")
            page.wait_for_timeout(600)
            after_notes = int(page.locator("#scoreMeta").inner_text().split(" notes")[0].split("·")[-1].strip())
            assert after_notes == before_notes + 1

            # Import MusicXML.
            fixture = ROOT / "examples/fixtures/fixture_050.musicxml"
            page.locator("#fileInput").set_input_files(str(fixture))
            page.wait_for_timeout(1000)
            assert page.locator("#scoreTitle").inner_text() == "Fixture 050"
            assert "28 notes" in page.locator("#scoreMeta").inner_text()

            # Export every supported format and verify the file signature.
            signatures = {
                "svg": b"<svg",
                "pdf": b"%PDF-1.4",
                "midi": b"MThd",
                "musicxml": b"<?xml",
            }
            with tempfile.TemporaryDirectory() as directory:
                for kind, signature in signatures.items():
                    with page.expect_download(timeout=20000) as download_info:
                        page.locator(f'[data-export="{kind}"]').click()
                    download = download_info.value
                    path = Path(directory) / download.suggested_filename
                    download.save_as(path)
                    data = path.read_bytes()
                    assert signature in data[:200], f"{kind} signature"
                    assert len(data) > 100, f"{kind} is too small"

            assert not console_errors, console_errors
            print("studio playwright: import, edit, preview and export all passed")
            browser.close()
        return 0
    finally:
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()


if __name__ == "__main__":
    raise SystemExit(main())
