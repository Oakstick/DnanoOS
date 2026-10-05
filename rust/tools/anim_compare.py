"""Old and new side by side: Ragnar at the same moments animated by the code (left) and by his clips
(right), each moment one picture (`--anim-compare`), in the gym's Sparring Ring where his fighting
goes by frame data; each moment seen close (from before him) and from the game's own chase camera
(behind and above him, as it is played). With `--also EXE2` (a build carrying other clips), each
picture has three panels: the code, EXE2's clips, EXE's clips. See ANIMATION.md.

    python tools/anim_compare.py OUT_DIR [EXE] [--also EXE2] [moment ...]

EXE defaults to the quick build (target/quick), then the release build. Needs only the Python
standard library.
"""
import os
import shutil
import struct
import subprocess
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NAME = 'AdventuresOfTheAncientBlades' + ('.exe' if os.name == 'nt' else '')
SIZE = '640x560'
RING = '0:ring:1'
GO = 3.0  # the bout's READY... GO! is over by then
CLOSE, PLAY = (30, 2.4), None  # before him, three-quarters; the game's chase camera

# name, script after the ring is set up, seconds, camera (degrees round from his front and
# distance, or None for the game's camera)
BASE = [
    ('idle', '', GO, CLOSE),
    ('idle_breath', '', GO + 0.6, CLOSE),
    ('idle_side', '', GO, (-90, 2.4)),
    ('idle_play', '', GO, PLAY),
    ('run_a', f'{GO - 0.8}:+up', GO, (-90, 2.6)),
    ('run_b', f'{GO - 0.8}:+up', GO + 0.12, (-90, 2.6)),
    ('run_play', f'{GO - 0.8}:+up', GO + 0.06, PLAY),
    ('walk_guarded', f'{GO - 0.8}:block,{GO - 0.8}:+up', GO + 0.1, (-60, 2.6)),
    ('first_windup', f'{GO}:attack', GO + 0.07, CLOSE),
    ('first_hit', f'{GO}:attack', GO + 0.12, CLOSE),
    ('first_hit_play', f'{GO}:attack', GO + 0.12, PLAY),
    ('first_through_play', f'{GO}:attack', GO + 0.18, PLAY),
    ('first_hit_side', f'{GO}:attack', GO + 0.12, (-90, 2.4)),
    ('first_settle', f'{GO}:attack', GO + 0.34, CLOSE),
    ('second_windup_play', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.31, PLAY),
    ('second_hit', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.35, CLOSE),
    ('second_hit_play', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.35, PLAY),
    ('second_hit_side', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.35, (-90, 2.4)),
    ('finisher_hit_play', f'{GO}:attack,{GO + 0.2}:attack,{GO + 0.42}:attack', GO + 0.62, PLAY),
    ('finisher_hit', f'{GO}:attack,{GO + 0.2}:attack,{GO + 0.42}:attack', GO + 0.62, CLOSE),
    ('block_raise', f'{GO}:block', GO + 0.04, CLOSE),
    ('block_held', f'{GO}:block', GO + 0.5, CLOSE),
    ('block_held_play', f'{GO}:block', GO + 0.5, PLAY),
    ('block_held_side', f'{GO}:block', GO + 0.5, (-90, 2.4)),
    ('jump_rise', f'{GO}:jump', GO + 0.1, (-80, 2.6)),
    ('jump_top', f'{GO}:jump', GO + 0.22, (-80, 2.6)),
    ('jump_top_play', f'{GO}:jump', GO + 0.22, PLAY),
    ('jump_land', f'{GO}:jump', GO + 0.5, (-80, 2.6)),
    # (Outside a fight his leap is the jump; in the ring it is the evade, the roll above.)
    ('leap_garden_top', '!0:dungeon:0,4:jump', 4.2, (-80, 2.6)),
    ('leap_garden_play', '!0:dungeon:0,4:jump', 4.2, PLAY),
]
MOMENTS = BASE


def decode(path):
    """An 8-bit RGBA PNG with unfiltered rows (as the game's captures are): (width, height, rows)."""
    data = open(path, 'rb').read()
    pos, idat, w, h = 8, b'', 0, 0
    while pos < len(data):
        n, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b'IHDR':
            w, h = struct.unpack('>II', body[:8])
        elif kind == b'IDAT':
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * 4 + 1
    return w, h, [raw[y * stride + 1:(y + 1) * stride] for y in range(h)]


def encode(path, w, h, rows):
    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    raw = b''.join(b'\0' + r for r in rows)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 6)) + chunk(b'IEND', b'')
    open(path, 'wb').write(png)


def capture(exe, png, script, t, cam, save):
    shutil.rmtree(save, ignore_errors=True)
    os.makedirs(save)
    env = dict(os.environ, VH_SAVE_DIR=save)
    env.pop('VH_ANIM', None)
    # (A script starting with ! is played as it is, outside the ring.)
    full = script[1:] if script.startswith('!') else RING + (',' + script if script else '')
    if cam is None:
        # The chase camera (behind him), and he turned away from it, up the ring, before he acts.
        turn = '0.5:camera' if script.startswith('!') else f'0.5:camera,{GO - 0.6}:+up,{GO - 0.5}:-up'
        full = full + ',' + turn if '+up' not in full else full + ',0.5:camera'
    args = [exe, '--capture', png, '--size', SIZE, '--time', f'{t:.3f}', '--script', full, '--no-hud', '--anim-compare']
    if cam:
        args += ['--closeup', str(cam[0]), str(cam[1])]
    run = subprocess.run(args, env=env, capture_output=True, text=True)
    if run.returncode:
        sys.exit(f'{png}: capture failed\n{run.stderr}')


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    out = args.pop(0)
    also = None
    if '--also' in args:
        i = args.index('--also')
        also = args[i + 1]
        del args[i:i + 2]
    exe = args[0] if args and os.path.isfile(args[0]) else None
    names = [a for a in args if not os.path.isfile(a)]
    if exe is None:
        for prof in ('quick', 'release'):
            p = os.path.join(ROOT, 'target', prof, NAME)
            if os.path.exists(p):
                exe = p
                break
    if exe is None:
        sys.exit('no build: cargo build --profile quick')
    os.makedirs(out, exist_ok=True)
    save = os.path.join(out, 'save')
    for name, script, t, cam in MOMENTS:
        if names and name not in names:
            continue
        png = os.path.join(out, name + '.png')
        capture(exe, png, script, t, cam, save)
        if also:
            other = os.path.join(out, name + '.other.png')
            capture(also, other, script, t, cam, save)
            w, h, new = decode(png)
            _, _, old = decode(other)
            half = w // 2 * 4
            # The code (from either: the same), the other build's clips, this one's.
            encode(png, w // 2 * 3, h, [old[y][:half] + old[y][half:] + new[y][half:] for y in range(h)])
            os.remove(other)
        print(f'{name:20} {png}')
    shutil.rmtree(save, ignore_errors=True)


main()
