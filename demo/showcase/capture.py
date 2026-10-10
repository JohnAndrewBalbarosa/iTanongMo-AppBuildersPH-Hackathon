"""Capture the real static frontend with an entirely in-memory API."""

import json
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
from urllib.parse import urlparse

from fixtures import DESCRIPTION, REFERENCE, REPLY, TITLE, RecordingAPI
from playwright.sync_api import sync_playwright
from story import ROOT, WORK


def capture_screens():
    # Mental model: serve local assets, intercept every API call, then record the complete flow.
    WORK.mkdir(parents=True, exist_ok=True)
    api = RecordingAPI()
    server = ThreadingHTTPServer(
        ("127.0.0.1", 0), partial(QuietHandler, directory=str(ROOT / "frontend"))
    )
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel="chrome", headless=True)
            try:
                page = browser.new_page(
                    viewport={"width": 1440, "height": 810},
                    device_scale_factor=1,
                    reduced_motion="reduce",
                )
                page.set_default_timeout(10000)
                errors = []
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.route("**/*", partial(serve_recording_request, api=api))
                origin = f"http://127.0.0.1:{server.server_port}"
                page.goto(f"{origin}/index.html")
                page.wait_for_load_state("networkidle")
                page.locator(".site-header").wait_for()
                page.locator('link[href="assets/site.css"]').wait_for(state="attached")
                page.screenshot(path=str(WORK / "home.png"))
                capture_citizen(page, origin)
                api.role = "government_officer"
                capture_government(page, origin)
                api.role = "requester"
                page.goto(f"{origin}/citizen.html")
                page.wait_for_load_state("networkidle")
                page.locator("#citizen-request-detail .timeline-item").last.wait_for()
                frame_portal(page)
                frame_element(page, "#citizen-request-detail")
                page.screenshot(path=str(WORK / "tracking.png"))
                if errors:
                    raise RuntimeError(f"Frontend recording errors: {errors[:3]}")
                (WORK / "capture.json").write_text(
                    json.dumps(
                        {
                            "page_errors": errors,
                            "submitted": api.submitted,
                            "replied": api.replied,
                            "live_api_calls": 0,
                            "frontend": "repository static files",
                        }
                    ),
                    encoding="utf-8",
                )
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def serve_recording_request(route, api):
    request = route.request
    url = urlparse(request.url)
    if url.path == "/config.js":
        route.fulfill(
            content_type="application/javascript",
            body="window.ITANONGMO_CONFIG = Object.freeze({apiBaseUrl: window.location.origin});",
        )
    elif url.path == "/assets/showcase-recording.css":
        route.fulfill(
            content_type="text/css",
            body=(ROOT / "demo/showcase/recording.css").read_text(encoding="utf-8"),
        )
    elif url.path.startswith("/api/v1/"):
        headers = {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Headers": "Content-Type, X-CSRF-Token",
            "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
        }
        if request.method == "OPTIONS":
            route.fulfill(status=204, headers=headers)
            return
        body = json.loads(request.post_data or "{}")
        result = api.respond(url.path.removeprefix("/api/v1"), request.method, body)
        route.fulfill(
            content_type="application/json",
            body=json.dumps({"data": result}),
            headers=headers,
        )
    elif url.hostname == "127.0.0.1" and url.port != 1:
        route.continue_()
    else:
        route.abort()


def capture_citizen(page, origin):
    page.goto(f"{origin}/citizen.html")
    page.wait_for_load_state("networkidle")
    page.locator("#citizen-agency option").wait_for(state="attached")
    frame_portal(page)
    page.locator("#citizen-requester-email").fill("alex@example.org")
    page.locator("#citizen-request-purpose").fill(
        "Para sa paghahanda ng scholarship application."
    )
    page.locator('input[name="consent"]').check()
    page.locator("#citizen-request-title").focus()
    capture_typing(page, "#citizen-request-title", TITLE, "title")
    capture_typing(page, "#citizen-request-details", DESCRIPTION, "details")
    page.screenshot(path=str(WORK / "typed.png"))
    page.locator('#citizen-request-form button[type="submit"]').click()
    page.locator("#citizen-submit-status").filter(has_text=REFERENCE).wait_for()
    page.locator("#citizen-request-detail h3").wait_for()
    frame_element(page, "#citizen-request-detail")
    page.screenshot(path=str(WORK / "submitted.png"))


def capture_government(page, origin):
    page.goto(f"{origin}/government-demo.html")
    page.wait_for_load_state("networkidle")
    page.locator(".case-card").first.wait_for()
    page.locator("#government-case-title").filter(has_text=REFERENCE).wait_for()
    frame_portal(page)
    frame_element(page, ".board-panel")
    page.screenshot(path=str(WORK / "board.png"))
    page.locator("#government-case-panel").scroll_into_view_if_needed()
    page.evaluate(
        "window.scrollTo(0, document.querySelector('#government-case-panel').getBoundingClientRect().top + window.scrollY - 100)"
    )
    capture_typing(page, "#government-reply-demo", REPLY, "reply")
    page.locator('#government-reply-form button[type="submit"]').click()
    page.locator("#government-action-status").filter(
        has_text="Public response sent"
    ).wait_for()
    page.screenshot(path=str(WORK / "reply-sent.png"))


def capture_typing(page, selector, text, name):
    field = page.locator(selector)
    field.focus()
    field.evaluate(
        "element => element.scrollIntoView({block: 'center', behavior: 'instant'})"
    )
    # Capture every two characters. At 24 fps the repeated frames read as natural typing.
    lengths = list(range(0, len(text), 2)) + [len(text)]
    for index, length in enumerate(lengths):
        field.fill(text[:length])
        page.screenshot(path=str(WORK / f"{name}-{index:03}.png"))
    (WORK / f"{name}.json").write_text(
        json.dumps(
            {
                "frames": len(lengths),
                "characters": len(text),
                "field": field.bounding_box(),
            }
        ),
        encoding="utf-8",
    )


def frame_portal(page):
    page.evaluate("""() => {
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = 'assets/showcase-recording.css';
        document.head.append(link);
        const consent = document.querySelector('input[name="consent"]');
        if (consent) for (const node of consent.parentElement.childNodes) {
            if (node.nodeType === Node.TEXT_NODE) node.textContent = node.textContent.replace('demo privacy notice', 'privacy notice');
        }
    }""")
    page.wait_for_function(
        "() => Array.from(document.styleSheets).some(sheet => sheet.href?.endsWith('showcase-recording.css'))"
    )


def frame_element(page, selector):
    page.locator(selector).evaluate(
        "element => window.scrollTo({top: element.getBoundingClientRect().top + window.scrollY - 100, behavior: 'instant'})"
    )


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *args):
        return


if __name__ == "__main__":
    capture_screens()
    print(json.dumps({"event": "showcase.capture.completed", "outcome": "success"}))
