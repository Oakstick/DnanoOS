"""The story's cast, code beside clips, at every key moment of Scene I and Scene II's embrace (ANIMATION.md
§12): one capture a moment (`--anim-compare`: the code's poses on the left, the clips' on the right),
and a contact sheet of them all, each moment a row, before and after.

    python tools/cast_captures.py OUT_DIR [EXE] [moment ...]

EXE defaults to the quick build. Writes OUT_DIR/<moment>.png (2560 x 720) and OUT_DIR/contact.png.
"""
import os
import subprocess
import sys

from PIL import Image, ImageDraw

# (name, script, what the moment is): `beat:N:S` is Scene I's beat N, S seconds in; `ending:S` is
# Scene II, S seconds in.
MOMENTS = [
    ("01_pass", "beat:1:2.75", "a pass (the fourth kick's contact)"),
    ("02_pass", "beat:1:3.5", "the fifth kick"),
    ("03_miskick", "beat:1:4.2", "the miskick, the ball away to the gate"),
    ("04_fetch", "beat:1:5.9", "the boy nearest runs after it"),
    ("05_peer", "beat:1:6.9", "he peers through the gate"),
    ("06_eyes", "beat:2:0.6", "the eyes in the fog: frozen, peering"),
    ("07_burst", "beat:2:1.4", "Krampus bursts the gate, reaching"),
    ("08_grab", "beat:2:1.62", "the grab: the boy heaved up"),
    ("09_drag", "beat:2:2.4", "back into the graves with him"),
    ("10_scream", "beat:3:1.5", "\"He took him!\""),
    ("11_sprint", "beat:4:1.0", "after him through the gate"),
    ("12_fall", "beat:5:0.55", "the fall through, tumbling out"),
    ("13_landing", "beat:5:1.2", "the hard landing"),
    ("14_getting_up", "beat:5:2.3", "picking themselves up"),
    ("15_dog_wakes", "beat:5:3.05", "the old man dozing; the dog rising"),
    ("16_dog_lunge", "beat:6:1.0", "the dog's lunge, the boys backing away"),
    ("17_calming", "beat:7:0.9", "\"Easy, Quitmer\": his hand out to the dog"),
    ("18_dog_down", "beat:7:2.6", "the dog lying down again"),
    ("19_old_man_talks", "beat:8:1.5", "the old man turns to the boys and talks"),
    ("20_boys_answer", "beat:9:1.2", "the boys answer"),
    ("21_looking_away", "beat:14:1.5", "\"tonight, of all nights, we slept\""),
    ("22_naming_souls", "beat:18:2.0", "he names the souls, his hand to the door"),
    ("23_we_will", "beat:19:1.0", "\"We will. For our friend.\""),
    ("24_promise", "beat:20:2.0", "his promise"),
    ("25_rising", "beat:21:2.6", "the door opens; he rises"),
    ("26_descent", "beat:21:5.0", "they go down; he sees them off"),
    ("27_run_to_him", "ending:1.8", "Scene II: the freed boy runs to him"),
    ("28_embrace", "ending:4.2", "the embrace"),
    ("29_gaze", "ending:11.9", "looking up into the light"),
    ("30_hand", "ending:14.2", "his hand held up"),
]


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    out = a[0]
    exe = a[1] if len(a) > 1 and a[1].endswith(".exe") else os.path.join("target", "quick", "AdventuresOfTheAncientBlades.exe")
    only = set(x for x in a[1:] if not x.endswith(".exe"))
    os.makedirs(out, exist_ok=True)
    env = dict(os.environ, VH_GPU_STRICT="1")
    shots = []
    for name, script, what in MOMENTS:
        path = os.path.join(out, name + ".png")
        if not only or name in only:
            subprocess.run([exe, "--capture", path, "--size", "1280x720", "--time", "0.05", "--script", "0:" + script, "--anim-compare"],
                           env=env, check=True, capture_output=True)
            print("cast_captures:", name, what)
        shots.append((path, what))
    # The contact sheet: each moment a row, the code's half and the clips' half side by side.
    w, h = 640, 180
    sheet = Image.new("RGB", (w, h * len(shots) + 24 * len(shots)), (16, 16, 18))
    d = ImageDraw.Draw(sheet)
    for k, (path, what) in enumerate(shots):
        if not os.path.exists(path):
            continue
        im = Image.open(path).convert("RGB").resize((w, h))
        y = k * (h + 24)
        d.text((6, y + 5), f"{os.path.basename(path)[:-4]}  {what}   (code | clips)", fill=(220, 220, 210))
        sheet.paste(im, (0, y + 24))
    sheet.save(os.path.join(out, "contact.png"))
    print("cast_captures: contact.png")


if __name__ == "__main__":
    main()
