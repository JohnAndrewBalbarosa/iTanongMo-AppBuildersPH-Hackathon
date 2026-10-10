"""CPU-only 1080p film: browser captures, paced typing, music, and SRT."""

import argparse
import json
import subprocess
import wave
from functools import lru_cache

import imageio_ffmpeg
import numpy as np
from capture import capture_screens
from PIL import Image, ImageDraw, ImageFont
from story import DURATION, FPS, OUTPUT, SCENES, SIZE, WORK, write_narration

SCREEN_ORIGIN = (280, 150)
SCREEN_SIZE = (1360, 765)
CYAN = (103, 217, 213)
WHITE = (240, 248, 252)
MUTED = (147, 173, 190)


def main():
    # Mental model: record isolated frontend states, render the shared timeline, then mux music.
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reuse-captures", action="store_true")
    args = parser.parse_args()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)
    write_narration()
    if not args.reuse_captures:
        capture_screens()
    compose_soundtrack()
    encode_film()
    write_contact_sheet()
    print(
        json.dumps(
            {
                "event": "showcase.render.completed",
                "outcome": "success",
                "duration_seconds": DURATION,
                "resolution": list(SIZE),
                "fps": FPS,
                "output": str(OUTPUT / "itanongmo-showcase.mp4"),
            }
        )
    )


def compose_soundtrack():
    sample_rate = 44100
    count = sample_rate * DURATION
    time = np.arange(count, dtype=np.float32) / sample_rate
    music = np.zeros(count, dtype=np.float32)
    # Original, quiet ambient score in D major; no third-party music or voice service.
    chords = [
        (146.83, 185.0, 220.0),
        (110.0, 146.83, 185.0),
        (98.0, 146.83, 196.0),
        (110.0, 138.59, 164.81),
    ]
    for start in range(0, DURATION, 8):
        end = min(start + 9, DURATION)
        local = time[start * sample_rate : end * sample_rate] - start
        envelope = np.minimum(local / 1.2, 1) * np.minimum(
            (end - start - local) / 1.5, 1
        )
        for frequency in chords[(start // 8) % len(chords)]:
            note = np.sin(2 * np.pi * frequency * local) + 0.16 * np.sin(
                2 * np.pi * frequency * 2 * local
            )
            music[start * sample_rate : end * sample_rate] += 0.016 * envelope * note
    for beat, start in enumerate(np.arange(8, 81, 0.625)):
        sample_start = int(start * sample_rate)
        length = min(int(sample_rate * 0.45), count - sample_start)
        local = np.arange(length, dtype=np.float32) / sample_rate
        frequency = chords[(int(start) // 8) % 4][beat % 3] * 4
        music[sample_start : sample_start + length] += (
            0.012 * np.sin(2 * np.pi * frequency * local) * np.exp(-local * 10)
        )
    fade = np.minimum(time / 2, 1) * np.minimum((DURATION - time) / 3, 1)
    music *= fade
    stereo = np.stack((music, music * 0.94), axis=1)
    with wave.open(str(WORK / "soundtrack.wav"), "wb") as file:
        file.setnchannels(2)
        file.setsampwidth(2)
        file.setframerate(sample_rate)
        file.writeframes((stereo * 32767).astype("<i2").tobytes())


def encode_film():
    executable = imageio_ffmpeg.get_ffmpeg_exe()
    silent = WORK / "silent.mp4"
    command = [
        executable,
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-vcodec",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-s",
        "1920x1080",
        "-r",
        str(FPS),
        "-i",
        "-",
        "-an",
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "20",
        "-threads",
        "2",
        "-pix_fmt",
        "yuv420p",
        str(silent),
    ]
    with (WORK / "encode-error.log").open("wb") as error_log:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stderr=error_log)
        try:
            for frame in range(DURATION * FPS):
                process.stdin.write(compose_frame(frame / FPS).tobytes())
        finally:
            process.stdin.close()
        if process.wait() != 0:
            raise RuntimeError(
                "Video encode failed; inspect var/showcase/encode-error.log"
            )
    subprocess.run(
        [
            executable,
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            str(silent),
            "-i",
            str(WORK / "soundtrack.wav"),
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "160k",
            "-movflags",
            "+faststart",
            "-t",
            str(DURATION),
            str(OUTPUT / "itanongmo-showcase.mp4"),
        ],
        check=True,
    )


def compose_frame(time):
    scene = next(scene for scene in SCENES if scene[0] <= time < scene[1])
    start, end, visual, chapter, title, subtitle = scene
    local = time - start
    background = make_background().copy()
    draw = ImageDraw.Draw(background)
    draw_brand(draw, (80, 47), size=30)
    draw.text(
        (1840, 60), "APPBUILDERSPH  /  SHOWCASE", font=font(18), fill=MUTED, anchor="ra"
    )
    if visual in {"intro", "outro"}:
        draw_hero(background, local, end - start, title, subtitle, visual)
    else:
        draw.text((450, 60), chapter, font=font(18), fill=CYAN)
        draw_browser(background, select_capture(visual, local), local, visual)
        draw = ImageDraw.Draw(background)
        draw.text((960, 947), title, font=font(45, True), fill=WHITE, anchor="mt")
        draw.text((960, 1004), subtitle, font=font(22), fill=MUTED, anchor="mt")
    draw = ImageDraw.Draw(background)
    draw.line((80, 1058, 1840, 1058), fill=(34, 54, 67), width=2)
    draw.line((80, 1058, 80 + int(1760 * time / DURATION), 1058), fill=CYAN, width=3)
    # A short dip at each cut gives the next screen breathing room.
    fade_in = min(local / 0.42, 1)
    fade_out = min((end - time) / 0.32, 1) if visual != "outro" else 1
    opacity = max(0.0, min(fade_in, fade_out))
    if opacity < 1:
        background = Image.blend(
            Image.new("RGB", SIZE, (7, 17, 29)), background, opacity
        )
    return background


@lru_cache(maxsize=1)
def make_background():
    y, x = np.mgrid[: SIZE[1], : SIZE[0]]
    glow = np.exp(-((x - 1450) ** 2 / 650000 + (y - 330) ** 2 / 400000))
    glow_left = np.exp(-((x - 300) ** 2 / 500000 + (y - 850) ** 2 / 200000))
    pixels = np.empty((SIZE[1], SIZE[0], 3), dtype=np.uint8)
    for channel, base in enumerate((7, 17, 29)):
        pixels[:, :, channel] = (
            base + glow * (4, 25, 34)[channel] + glow_left * (5, 13, 14)[channel]
        )
    image = Image.fromarray(pixels)
    draw = ImageDraw.Draw(image)
    for x in range(0, 1920, 80):
        draw.line((x, 0, x, 1080), fill=(15, 31, 43))
    for y in range(0, 1080, 80):
        draw.line((0, y, 1920, y), fill=(15, 31, 43))
    draw.arc((1110, -350, 2290, 830), 30, 250, fill=(27, 64, 77), width=2)
    return image


@lru_cache(maxsize=16)
def font(size, bold=False):
    path = "C:/Windows/Fonts/seguisb.ttf" if bold else "C:/Windows/Fonts/segoeui.ttf"
    return ImageFont.truetype(path, size)


def draw_brand(draw, position, size):
    x, y = position
    draw.rounded_rectangle((x, y, x + size + 10, y + size + 10), radius=12, fill=CYAN)
    draw.text(
        (x + (size + 10) / 2, y + 1),
        "?",
        font=font(size, True),
        fill=(7, 25, 36),
        anchor="mt",
    )
    draw.text((x + size + 24, y), "iTanongMo", font=font(size, True), fill=WHITE)


def draw_hero(image, local, duration, title, subtitle, visual):
    draw = ImageDraw.Draw(image)
    offset = int(24 * (1 - min(local / 1.4, 1)) ** 3)
    draw.text(
        (960, 257 + offset), "ITANONGMO", font=font(24, True), fill=CYAN, anchor="mt"
    )
    draw.text(
        (960, 348 + offset),
        title,
        font=font(108 if visual == "intro" else 82, True),
        fill=WHITE,
        anchor="mt",
    )
    draw.text((960, 500 + offset), subtitle, font=font(32), fill=MUTED, anchor="mt")
    chips = ["PUBLIC RECORDS", "REQUEST TRACKING", "AGENCY WORKSPACE"]
    for index, label in enumerate(chips):
        if local > 1.0 + index * 0.4:
            x = 472 + index * 336
            draw.rounded_rectangle(
                (x, 630, x + 304, 689),
                radius=29,
                outline=(51, 99, 111),
                width=2,
                fill=(12, 37, 49),
            )
            draw.text(
                (x + 152, 648), label, font=font(17, True), fill=CYAN, anchor="mt"
            )
    if visual == "outro":
        draw.text(
            (960, 818),
            "Isang tanong. Isang mas malinaw na susunod na hakbang.",
            font=font(26),
            fill=WHITE,
            anchor="mt",
        )


def select_capture(visual, local):
    if visual == "typing":
        if local < 1.2:
            return "title-000.png"
        if local < 5.8:
            return typing_capture("title", (local - 1.2) / 4.6)
        if local < 7.2:
            return typing_capture("title", 1)
        if local < 13.7:
            return typing_capture("details", (local - 7.2) / 6.5)
        return "typed.png"
    if visual == "reply":
        if local < 1.5:
            return "reply-000.png"
        if local < 9.5:
            return typing_capture("reply", (local - 1.5) / 8)
        if local < 11:
            return typing_capture("reply", 1)
        return "reply-sent.png"
    return f"{visual}.png"


def typing_capture(name, fraction):
    count = typing_metadata(name)["frames"]
    index = min(count - 1, max(0, int(fraction * (count - 1))))
    return f"{name}-{index:03}.png"


@lru_cache(maxsize=3)
def typing_metadata(name):
    return json.loads((WORK / f"{name}.json").read_text(encoding="utf-8"))


@lru_cache(maxsize=6)
def screenshot(name):
    with Image.open(WORK / name) as image:
        return image.convert("RGB").resize(SCREEN_SIZE, Image.Resampling.LANCZOS)


def draw_browser(image, capture, local, visual):
    draw = ImageDraw.Draw(image)
    x, y = SCREEN_ORIGIN
    y += int(18 * (1 - min(local / 0.8, 1)) ** 3)
    width, height = SCREEN_SIZE
    draw.rounded_rectangle(
        (x - 14, y - 48, x + width + 14, y + height + 14),
        radius=22,
        fill=(3, 11, 18),
        outline=(47, 87, 101),
        width=2,
    )
    draw.rounded_rectangle((x, y - 40, x + width, y), radius=13, fill=(23, 42, 55))
    for index, color in enumerate(((252, 109, 108), (248, 195, 94), (97, 206, 146))):
        cx = x + 20 + index * 22
        draw.ellipse((cx, y - 28, cx + 10, y - 18), fill=color)
    draw.text(
        (960, y - 29),
        "iTanongMo  /  Public information portal",
        font=font(16),
        fill=MUTED,
        anchor="mt",
    )
    image.paste(screenshot(capture), (x, y))
    if visual in {"typing", "reply"}:
        name = "reply" if visual == "reply" else "title" if local < 7.2 else "details"
        field = typing_metadata(name)["field"]
        # Keep the pointer beside the active field while the actual captured input types.
        px = x + (field["x"] + field["width"] - 26) * width / 1440
        py = y + (field["y"] + 20) * height / 810
        draw_pointer(image, px, py)


def draw_pointer(image, x, y):
    draw = ImageDraw.Draw(image)
    draw.ellipse((x - 15, y - 15, x + 15, y + 15), outline=(33, 142, 183), width=2)
    draw.polygon(
        ((x, y), (x + 4, y + 27), (x + 11, y + 19), (x + 21, y + 20)),
        fill=WHITE,
        outline=(7, 17, 29),
        width=2,
    )


def write_contact_sheet():
    poster = compose_frame(4)
    poster.save(OUTPUT / "poster.jpg", quality=94)
    sheet = Image.new("RGB", (1280, 1080), (7, 17, 29))
    for index, scene in enumerate(SCENES):
        time = scene[0] + min(8, (scene[1] - scene[0]) / 2)
        frame = compose_frame(time).resize((640, 360), Image.Resampling.LANCZOS)
        # Three rows cover the core flow; opening and closing are separate in the poster/video.
        if 1 <= index <= 6:
            cell = index - 1
            sheet.paste(frame, ((cell % 2) * 640, (cell // 2) * 360))
    sheet.save(OUTPUT / "storyboard.jpg", quality=92)


if __name__ == "__main__":
    main()
