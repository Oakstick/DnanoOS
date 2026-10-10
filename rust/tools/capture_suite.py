"""The capture suite: a golden-image check for refactors.

Renders a fixed set of scripted captures (every stage and the main systems) with the release
build, then compares two renders. Captures are seeded, so the same build renders the same frame
every time, but the GPU still adds a little noise: a few dozen pixels a level or two apart. So a
render matches when every frame log (screen, instance counts, hero position) is identical and no
picture differs by more than MAX_PIXELS pixels or MAX_DELTA levels.

    python tools/capture_suite.py render OUT_DIR          # render the suite into OUT_DIR
    python tools/capture_suite.py compare BASE_DIR NEW_DIR  # exit 1 if they do not match

Typical use: render a baseline before a change, render again after, compare. Needs only the
Python standard library and a release build (`cargo build --release`)."""
import os, re, shutil, struct, subprocess, sys, zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(ROOT, 'target', 'release', 'AdventuresOfTheAncientBlades' + ('.exe' if os.name == 'nt' else ''))
SIZE = '640x360'
MAX_PIXELS = 600
MAX_DELTA = 20

# name, seconds to run, script, extra arguments
SUITE = [
    ('title', 2, '0:pause', []),
    ('garden', 7, '0:dungeon:0,4:attack,4.5:jump,5:bomb', []),
    ('garden_top', 6, '0:dungeon:0,4:camera,4.5:+right,5.5:-right', []),
    ('keys', 6, '0:dungeon:0,4:nugget,4.2:nugget,4.4:streak', []),
    ('gate', 8, '0:dungeon:0,4:gate', []),
    ('arena', 9, '0:boss,4.5:attack,6:wound:20,7:daze', []),
    ('slain', 12, '0:boss,4:slay', []),
    ('hub', 6, '0:hub,0.5:pause,1:confirm', []),
    ('hub_sins', 6, '0:sins:127,0.2:hub,0.7:pause,1.2:confirm', []),
    ('gym', 8, '0:gym', []),
    ('realm', 6, '0:realm:3', []),
    ('court', 7, '0:court:4,1.2:at:13:12,1.3:foes:1', []),
    ('court2', 7, '0:court:4,1.2:at:13:12,1.3:foes:2', []),
    ('vault', 6, '0:realm:4', []),
    ('vault_hall', 8, '0:realm:4,4:at:66:50,5:attack', []),
    ('vault_close', 7, '0:realm:4,4:at:66:50', ['--view', '66:0.3:50:30:7:0.45']),
    ('vault_toll', 6, '0:realm:4,4:gate', []),
    ('shop', 3, '0:shop', []),
    ('shop_row', 3, '0:coins:5000,0.1:runes:50,0.2:shop,1:row:2', []),
    ('map', 2, '0:map', []),
    ('pause', 6, '0:dungeon:0,4:pause', []),
    ('bless', 8, '0:dungeon:0,3:bless:2,4:angel', []),
    ('hurt', 6, '0:dungeon:0,4:hurt,4.1:hurt,4.3:potion:3', []),
    ('arm', 7, '0:arm:6,0.1:dungeon:0,4:attack', []),
    ('goo', 6, '0:dungeon:0,4:goo', []),
    ('tour', 4, '0:tour,0.1:dungeon:0', []),
]

LOG_LINE = re.compile(r'screen [A-Za-z]+, \d+ static instances, \d+ drawn this frame, hero at [-0-9.,]+')


def render(out):
    if not os.path.exists(EXE):
        sys.exit(f'no release build at {EXE}: run cargo build --release')
    os.makedirs(out, exist_ok=True)
    save = os.path.join(out, 'save')
    for name, seconds, script, extra in SUITE:
        # A fresh save for every capture, so none depends on the one before.
        shutil.rmtree(save, ignore_errors=True)
        os.makedirs(save)
        env = dict(os.environ, VH_SAVE_DIR=save)
        png = os.path.join(out, name + '.png')
        run = subprocess.run([EXE, '--capture', png, '--size', SIZE, '--time', str(seconds), '--script', script] + extra,
                             env=env, capture_output=True, text=True)
        log = LOG_LINE.search(run.stdout + run.stderr)
        if run.returncode or not log:
            sys.exit(f'{name}: capture failed\n{run.stderr}')
        with open(os.path.join(out, name + '.txt'), 'w') as f:
            f.write(log.group(0) + '\n')
        print(f'{name:12} {log.group(0)}')
    shutil.rmtree(save, ignore_errors=True)


def decode(path):
    """8-bit RGB or RGBA, non-interlaced PNG, as (width, height, bytes per pixel, pixels)."""
    data = open(path, 'rb').read()
    pos, idat, w, h, ctype = 8, b'', 0, 0, 6
    while pos < len(data):
        n, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b'IHDR':
            w, h, _, ctype = struct.unpack('>IIBB', body[:10])
        elif kind == b'IDAT':
            idat += body
        pos += 12 + n
    bpp = 4 if ctype == 6 else 3
    raw, stride = zlib.decompress(idat), w * bpp
    out, prev, i = bytearray(h * stride), bytearray(stride), 0
    for y in range(h):
        f = raw[i]
        line = bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b, c = prev[x], (prev[x - bpp] if x >= bpp else 0)
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return w, h, bpp, out


def pixel_diff(a, b):
    wa, ha, bpp, pa = decode(a)
    wb, hb, _, pb = decode(b)
    if (wa, ha) != (wb, hb):
        return None, None
    count = worst = 0
    for k in range(0, len(pa), bpp):
        d = max(abs(pa[k] - pb[k]), abs(pa[k + 1] - pb[k + 1]), abs(pa[k + 2] - pb[k + 2]))
        if d:
            count += 1
            worst = max(worst, d)
    return count, worst


def compare(base, new):
    ok = True
    for name, *_ in SUITE:
        la = open(os.path.join(base, name + '.txt')).read()
        lb = open(os.path.join(new, name + '.txt')).read()
        count, worst = pixel_diff(os.path.join(base, name + '.png'), os.path.join(new, name + '.png'))
        problems = []
        if la != lb:
            problems.append(f'log differs: {la.strip()}  vs  {lb.strip()}')
        if count is None:
            problems.append('size differs')
        elif count > MAX_PIXELS or worst > MAX_DELTA:
            problems.append(f'{count} pixels differ, by up to {worst}')
        if problems:
            ok = False
            print(f'{name:12} MISMATCH  ' + '; '.join(problems))
        else:
            print(f'{name:12} ok' + (f'  ({count} px of GPU noise, up to {worst})' if count else ''))
    print('MATCH' if ok else 'MISMATCH')
    return ok


if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == 'render':
        render(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == 'compare':
        sys.exit(0 if compare(sys.argv[2], sys.argv[3]) else 1)
    else:
        sys.exit(__doc__)
