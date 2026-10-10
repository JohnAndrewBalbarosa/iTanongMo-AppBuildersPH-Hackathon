"""One timeline owns scene cuts and the narration sidecar."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "demo" / "showcase" / "output"
WORK = ROOT / "var" / "showcase"
FPS = 24
DURATION = 90
SIZE = (1920, 1080)

# (start, end, visual, chapter, title, supporting line)
SCENES = [
    (
        0,
        8,
        "intro",
        "PUBLIC INFORMATION. PUBLIC EMPOWERMENT.",
        "May tanong ka?",
        "Simulan sa iTanongMo.",
    ),
    (
        8,
        18,
        "home",
        "01 / PARA SA MAMAMAYAN",
        "Impormasyon, mas abot-kamay.",
        "Isang malinaw na daan para humiling ng public records.",
    ),
    (
        18,
        34,
        "typing",
        "02 / BUUIN ANG REQUEST",
        "Mula tanong, tungo sa request.",
        "Piliin ang ahensiya. Ilahad ang kailangan. Suriin bago isumite.",
    ),
    (
        34,
        44,
        "submitted",
        "03 / MAY MALINAW NA REFERENCE",
        "Naisumite. Nasusubaybayan.",
        "May reference number at sariling timeline ang bawat request.",
    ),
    (
        44,
        56,
        "board",
        "04 / PARA SA AHENSIYA",
        "Organisadong pag-asikaso.",
        "New → Under review → For response",
    ),
    (
        56,
        69,
        "reply",
        "05 / TULOY ANG USAPAN",
        "Mas malinaw na tugon.",
        "Ihanda ang response at ibahagi ang susunod na hakbang.",
    ),
    (
        69,
        81,
        "tracking",
        "06 / BALIK SA MAMAMAYAN",
        "Bawat update, nasa isang lugar.",
        "Balikan ang request, status, at sagot ng ahensiya.",
    ),
    (
        81,
        90,
        "outro",
        "ITANONGMO / APPBUILDERSPH",
        "Magtanong. Subaybayan.",
        "Mas malinaw na ugnayan ng mamamayan at pamahalaan.",
    ),
]

# Short speaking windows leave silence between sentences and around scene cuts.
NARRATION = [
    (1.5, 5.5, "May tanong ka tungkol sa pampublikong impormasyon?"),
    (9.5, 15.5, "Sa iTanongMo, may malinaw na daan para humiling ng public records."),
    (19.5, 24.0, "Piliin ang ahensiya at ilagay ang pamagat ng iyong request."),
    (26.0, 31.5, "Isulat ang mga detalyeng kailangan, at suriin bago isumite."),
    (35.5, 41.0, "Pagkatapos magsumite, may reference number para sa pagsubaybay."),
    (
        45.5,
        53.0,
        "Para sa ahensiya, nasa isang board ang mga bagong request, kasalukuyang review, at ihahandang tugon.",
    ),
    (57.5, 61.5, "Maaaring iayos ang request at italaga sa tamang unit."),
    (63.0, 67.0, "Pagkatapos, magpadala ng malinaw na sagot sa mamamayan."),
    (
        70.5,
        78.5,
        "Sa sariling timeline, makikita ng mamamayan ang status, mga update, at sagot ng ahensiya.",
    ),
    (82.0, 85.0, "iTanongMo. Magtanong. Subaybayan."),
    (86.0, 89.0, "Mas malinaw na ugnayan sa pamahalaan."),
]


def write_narration():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    cues = []
    for index, (start, end, words) in enumerate(NARRATION, 1):
        cues.append(f"{index}\n{timestamp(start)} --> {timestamp(end)}\n{words}\n")
    (OUTPUT / "itanongmo-showcase.srt").write_text("\n".join(cues), encoding="utf-8")
    lines = [
        "# iTanongMo narration · 90 seconds",
        "",
        "Basahin ang bawat linya sa SRT window. Manahimik sa pagitan ng cues; hindi kailangang punuin ang bawat segundo.",
        "",
    ]
    lines.extend(
        f"- **{start:g}–{end:g}s:** {words}" for start, end, words in NARRATION
    )
    (OUTPUT / "narration.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def timestamp(seconds):
    milliseconds = round(seconds * 1000)
    hours, milliseconds = divmod(milliseconds, 3600000)
    minutes, milliseconds = divmod(milliseconds, 60000)
    whole_seconds, milliseconds = divmod(milliseconds, 1000)
    return f"{hours:02}:{minutes:02}:{whole_seconds:02},{milliseconds:03}"
