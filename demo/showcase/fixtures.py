"""Recording-only API responses. Never imported by the deployed frontend."""

from copy import deepcopy

REFERENCE = "FOI-2026-001042"
TITLE = "DOST scholarship program records for 2026"
DESCRIPTION = "Nais ko pong humiling ng listahan ng scholarship programs, eligibility requirements, at application dates para sa 2026."
REPLY = "Salamat sa inyong request. Kasalukuyang sinusuri ng Scholarship Unit ang mga rekord. Magbibigay kami ng update sa inyong timeline."
AGENCY = {
    "id": "dost",
    "name": "Department of Science and Technology",
    "acronym": "DOST",
}


class RecordingAPI:
    def __init__(self):
        self.role = "requester"
        self.submitted = False
        self.replied = False
        self.request = {
            "reference": REFERENCE,
            "title": TITLE,
            "description": DESCRIPTION,
            "agency": AGENCY,
            "agency_name": AGENCY["name"],
            "status": "submitted",
            "due_at": "2026-10-30T08:00:00Z",
            "assigned_unit_id": "scholarships",
        }

    # Mental model: route a request to a bounded fixture, updating only this recording's state.
    def respond(self, path, method, body):
        if path == "/auth/session":
            return {
                "actor": {"actor_type": self.role, "display_name": "Alex"},
                "csrf_token": "recording-only",
            }
        if path == "/agencies":
            return [AGENCY]
        if path == "/submissions" and method == "POST":
            self.submitted = True
            return {
                "batch_reference": "BATCH-2026-0042",
                "requests": [{"reference": REFERENCE}],
            }
        if path == "/citizen/requests":
            return {"requests": [self.citizen_request()] if self.submitted else []}
        if path.startswith("/citizen/requests/"):
            return {
                "request": self.citizen_request(),
                "timeline": {"events": self.events(), "messages": self.messages()},
            }
        if path == "/government/board":
            return {"cards": self.board_cards()}
        if path.endswith("/messages") and method == "POST":
            if body.get("body") != REPLY:
                raise ValueError("Unexpected recording response text")
            self.replied = True
            return {"saved": True}
        if path.endswith("/triage") and method == "POST":
            return {"saved": True}
        if path.startswith("/government/requests/"):
            return {
                "request": self.citizen_request(),
                "units": [{"id": "scholarships", "name": "Scholarship Unit"}],
                "actions": ["route", "reply"],
                "events": self.events(),
            }
        raise ValueError(f"Unimplemented recording route: {method} {path}")

    def citizen_request(self):
        result = deepcopy(self.request)
        result["status"] = "in_review" if self.replied else "submitted"
        result["latest_public_message"] = REPLY if self.replied else None
        return result

    def events(self):
        events = [
            {
                "type": "submitted",
                "summary": "Request received by DOST.",
                "created_at": "2026-10-10T08:00:00Z",
            }
        ]
        if self.replied:
            events.append(
                {
                    "type": "in_review",
                    "summary": "Assigned to the Scholarship Unit for review.",
                    "created_at": "2026-10-12T08:00:00Z",
                }
            )
        return events

    def messages(self):
        return (
            [
                {
                    "actor": "DOST Scholarship Unit",
                    "body": REPLY,
                    "created_at": "2026-10-12T09:00:00Z",
                }
            ]
            if self.replied
            else []
        )

    def board_cards(self):
        titles = [
            TITLE,
            "Community research grants",
            "Regional innovation centers",
            "Science education programs",
            "Research funding guidelines",
            "Technology transfer reports",
        ]
        statuses = [
            "submitted",
            "submitted",
            "in_review",
            "in_review",
            "granted",
            "partially_granted",
        ]
        return [
            {
                "reference": REFERENCE if index == 0 else f"FOI-2026-{1042 + index:06}",
                "title": title,
                "status": status,
                "column": status.replace("_", " ").title(),
                "unit_name": "Scholarship Unit" if index == 0 else "Records Unit",
                "due_at": self.request["due_at"],
            }
            for index, (title, status) in enumerate(zip(titles, statuses, strict=True))
        ]
