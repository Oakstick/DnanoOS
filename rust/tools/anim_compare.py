"""Old and new side by side: Ragnar at the same moments animated by the code (left) and by his clips
(right), each moment one picture (`--anim-compare`), in the gym's Sparring Ring where his fighting
goes by frame data. See ANIMATION.md.

    python tools/anim_compare.py OUT_DIR [EXE] [moment ...]

EXE defaults to the quick build (target/quick), then the release build. Needs only the Python
standard library.
"""
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NAME = 'AdventuresOfTheAncientBlades' + ('.exe' if os.name == 'nt' else '')
SIZE = '720x640'
RING = '0:ring:1'
GO = 3.0  # the bout's READY... GO! is over by then

# name, script after the ring is set up, seconds, camera (degrees round from his front, distance)
MOMENTS = [
    ('idle', '', GO, (30, 2.3)),
    ('idle_breath', '', GO + 0.6, (30, 2.3)),
    ('run_a', f'{GO - 0.8}:+up', GO, (-90, 2.6)),
    ('run_b', f'{GO - 0.8}:+up', GO + 0.12, (-90, 2.6)),
    ('run_c', f'{GO - 0.8}:+up', GO + 0.24, (-40, 2.6)),
    ('first_windup', f'{GO}:attack', GO + 0.07, (30, 2.4)),
    ('first_hit', f'{GO}:attack', GO + 0.12, (30, 2.4)),
    ('first_through', f'{GO}:attack', GO + 0.2, (30, 2.4)),
    ('first_settle', f'{GO}:attack', GO + 0.34, (30, 2.4)),
    ('second_windup', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.31, (30, 2.4)),
    ('second_hit', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.35, (30, 2.4)),
    ('second_through', f'{GO}:attack,{GO + 0.2}:attack', GO + 0.43, (30, 2.4)),
    ('block_raise', f'{GO}:block', GO + 0.04, (30, 2.4)),
    ('block_held', f'{GO}:block', GO + 0.5, (30, 2.4)),
    ('jump_rise', f'{GO}:jump', GO + 0.1, (-80, 2.6)),
    ('jump_top', f'{GO}:jump', GO + 0.22, (-80, 2.6)),
    ('jump_fall', f'{GO}:jump', GO + 0.36, (-80, 2.6)),
    ('jump_land', f'{GO}:jump', GO + 0.5, (-80, 2.6)),
    ('behind_run', f'{GO - 0.8}:+up', GO + 0.06, (180, 2.8)),
    ('behind_swing', f'{GO}:attack', GO + 0.14, (165, 2.8)),
]


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    out = sys.argv[1]
    exe = sys.argv[2] if len(sys.argv) > 2 and os.path.isfile(sys.argv[2]) else None
    names = [a for a in sys.argv[2:] if not os.path.isfile(a)]
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
    for name, script, t, (deg, dist) in MOMENTS:
        if names and name not in names:
            continue
        shutil.rmtree(save, ignore_errors=True)
        os.makedirs(save)
        env = dict(os.environ, VH_SAVE_DIR=save)
        env.pop('VH_ANIM', None)
        png = os.path.join(out, name + '.png')
        full = RING + (',' + script if script else '')
        run = subprocess.run([exe, '--capture', png, '--size', SIZE, '--time', f'{t:.3f}', '--script', full,
                              '--closeup', str(deg), str(dist), '--no-hud', '--anim-compare'], env=env, capture_output=True, text=True)
        if run.returncode:
            sys.exit(f'{name}: capture failed\n{run.stderr}')
        print(f'{name:14} {png}')
    shutil.rmtree(save, ignore_errors=True)


main()
