"""Installs and enables the Blender add-ons the pipeline uses, headless and idempotent.

Run it with Blender itself (it needs `bpy`), in the background, with online access allowed:

    blender -b --online-mode --python tools/blender/setup_addons.py -- [options]

Options (after the `--`):
    --dry-run              report what is installed and what would be done; change nothing
    --include-interactive  also install add-ons that only work in the UI (BlenderKit)
    --machin3-zip PATH     install MACHIN3tools from a zip you downloaded yourself
    --cache DIR            where downloads are kept (default: the system temp folder)

What it handles, by Blender version (`bpy.app.version`):
    * Blender 4.2 and later (the Extensions Platform):
        - bundled core add-ons, enabled: Node Wrangler, Rigify;
        - from the official repository (extensions.blender.org, `bpy.ops.extensions.*`):
          LoopTools, Extra Mesh Objects, Extra Curve Objects, Auto Mirror, F2, EdgeFlow;
    * Blender before 4.2 (legacy add-ons, `bpy.ops.preferences.addon_*`):
        - bundled, enabled: Node Wrangler, LoopTools, Rigify, Extra Objects (mesh and curve),
          Auto Mirror, F2;
        - EdgeFlow from its author's GitHub release;
    * any version:
        - BlenderGIS from its latest GitHub release (domlysz/BlenderGIS), as a legacy add-on;
        - BlenderKit is skipped unless --include-interactive (it needs a login, its own background
          client and the UI: it does nothing headless);
        - MACHIN3tools is skipped unless --machin3-zip: it has no verified public download (it is
          sold, with a free option, through machin3.io and Superhive), and it is pie menus and UI
          tools that do nothing headless.

Every step logs [SUCCESS], [SKIPPED] or [FAILED]; an add-on already installed and enabled is
left alone and nothing is downloaded again. The preferences are saved at the end
(`bpy.ops.wm.save_userpref`). The exit code is 1 if anything failed.
"""

import argparse
import json
import os
import shutil
import sys
import tempfile
import urllib.request
import zipfile

import addon_utils
import bpy

MODERN = bpy.app.version >= (4, 2, 0)
OFFICIAL_REPO = "blender_org"
USER_AGENT = "void-harvest-addon-setup/1.0 (+https://www.blender.org)"
TIMEOUT = 60

RESULTS = []


def log(status, name, detail=""):
    line = f"[{status}] {name}" + (f": {detail}" if detail else "")
    print(line, flush=True)
    RESULTS.append((status, name, detail))


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(prog="setup_addons.py")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--include-interactive", action="store_true")
    p.add_argument("--machin3-zip")
    p.add_argument("--cache", default=os.path.join(tempfile.gettempdir(), "blender-addon-cache"))
    return p.parse_args(argv)


# ---------------------------------------------------------------- what is installed

def module_names():
    return {m.__name__ for m in addon_utils.modules(refresh=True)}


def is_enabled(module):
    return addon_utils.check(module)[1]


def enable(name, module, dry):
    """Enables an installed module; logs the outcome. True when it is enabled afterwards."""
    if is_enabled(module):
        log("SKIPPED", name, f"already enabled ({module})")
        return True
    if dry:
        log("SKIPPED", name, f"would enable {module} (dry run)")
        return True
    try:
        if MODERN:
            addon_utils.enable(module, default_set=True, persistent=True, handle_error=None)
        else:
            bpy.ops.preferences.addon_enable(module=module)
        if is_enabled(module):
            log("SUCCESS", name, f"enabled {module}")
            return True
        log("FAILED", name, f"{module} did not enable (see the console above for its error)")
    except Exception as e:  # noqa: BLE001 - one add-on's failure must not stop the others
        log("FAILED", name, f"enabling {module}: {e}")
    return False


# ---------------------------------------------------------------- downloads

def online(name):
    """Downloads only with online access allowed (Blender's own switch: --online-mode)."""
    if bpy.app.online_access:
        return True
    log("FAILED", name, "online access is off: run Blender with --online-mode (or allow it in Preferences > System > Network)")
    return False


def fetch(url, cache, filename):
    """Downloads `url` into the cache once; returns the local path."""
    os.makedirs(cache, exist_ok=True)
    path = os.path.join(cache, filename)
    if os.path.isfile(path) and os.path.getsize(path) > 0:
        return path
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/octet-stream"})
    tmp = path + ".part"
    with urllib.request.urlopen(req, timeout=TIMEOUT) as r, open(tmp, "wb") as f:  # proxies come from HTTPS_PROXY
        shutil.copyfileobj(r, f)
    os.replace(tmp, path)
    return path


def github_latest(repo):
    """The latest release of `repo` (owner/name): its tag, its assets, its source zip."""
    req = urllib.request.Request(f"https://api.github.com/repos/{repo}/releases/latest", headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=TIMEOUT) as r:
        return json.load(r)


def user_addons_dir():
    path = bpy.utils.user_resource("SCRIPTS", path="addons", create=True)
    os.makedirs(path, exist_ok=True)
    return path


def has_manifest(zip_path):
    with zipfile.ZipFile(zip_path) as z:
        return any(n.endswith("blender_manifest.toml") for n in z.namelist())


def install_folder_zip(zip_path, folder_name):
    """Unpacks a source zip whose top folder is named for its commit (GitHub's) as `folder_name`
    in the user add-ons directory, replacing an older copy."""
    dest = os.path.join(user_addons_dir(), folder_name)
    with zipfile.ZipFile(zip_path) as z:
        tops = {n.split("/", 1)[0] for n in z.namelist() if "/" in n}
        if len(tops) != 1:
            raise RuntimeError(f"expected one top folder in {zip_path}, found {sorted(tops)}")
        top = tops.pop()
        staging = tempfile.mkdtemp(prefix="addon-")
        try:
            z.extractall(staging)
            if os.path.isdir(dest):
                shutil.rmtree(dest)
            shutil.move(os.path.join(staging, top), dest)
        finally:
            shutil.rmtree(staging, ignore_errors=True)
    return dest


def install_zip(zip_path):
    """Installs an add-on zip the way it was packaged: an extension (it has a manifest, 4.2+) into
    the user extensions repository, else a legacy add-on."""
    if MODERN and has_manifest(zip_path):
        bpy.ops.extensions.package_install_files(filepath=zip_path, repo="user_default", enable_on_install=False, overwrite=True)
    else:
        bpy.ops.preferences.addon_install(filepath=zip_path, overwrite=True)


# ---------------------------------------------------------------- each kind of add-on

def bundled(name, module, dry):
    if module not in module_names():
        log("FAILED", name, f"{module} is not part of this Blender build ({bpy.app.version_string})")
        return
    enable(name, module, dry)


def official_repo_index():
    for i, repo in enumerate(bpy.context.preferences.extensions.repos):
        if repo.module == OFFICIAL_REPO:
            return i, repo
    return None, None


_SYNCED = False


def extension(name, pkg_id, dry):
    """An extension from extensions.blender.org: installed once, then enabled."""
    module = f"bl_ext.{OFFICIAL_REPO}.{pkg_id}"
    if module in module_names():
        enable(name, module, dry)
        return
    if dry:
        log("SKIPPED", name, f"would install {pkg_id} from extensions.blender.org (dry run)")
        return
    if not online(name):
        return
    index, repo = official_repo_index()
    if repo is None:
        log("FAILED", name, "the official extensions repository is missing from Preferences > Get Extensions")
        return
    global _SYNCED
    try:
        if not repo.enabled:
            repo.enabled = True
        if not _SYNCED:
            bpy.ops.extensions.repo_sync(repo_index=index)
            _SYNCED = True
        bpy.ops.extensions.package_install(repo_index=index, pkg_id=pkg_id, enable_on_install=True)
    except Exception as e:  # noqa: BLE001
        # The command line does the same work outside this session.
        try:
            import subprocess

            cmd = [bpy.app.binary_path, "--online-mode", "-c", "extension", "install", "-s", "-e", pkg_id]
            done = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
            if done.returncode != 0:
                raise RuntimeError(done.stderr.strip() or done.stdout.strip())
            addon_utils.modules(refresh=True)
        except Exception as e2:  # noqa: BLE001
            log("FAILED", name, f"installing {pkg_id}: {e}; command line: {e2}")
            return
    if module in module_names():
        log("SUCCESS", name, f"installed {pkg_id}")
        enable(name, module, dry)
    else:
        log("FAILED", name, f"{pkg_id} did not appear after installing")


def edgeflow_legacy(args):
    """EdgeFlow before 4.2: its author's GitHub release (BenjaminSauder/EdgeFlow)."""
    name, module = "EdgeFlow", "EdgeFlow"
    if module in module_names():
        enable(name, module, args.dry_run)
        return
    if args.dry_run:
        log("SKIPPED", name, "would install the latest GitHub release (dry run)")
        return
    if not online(name):
        return
    try:
        rel = github_latest("BenjaminSauder/EdgeFlow")
        zips = [a for a in rel.get("assets", []) if a["name"].endswith(".zip")]
        if zips:
            path = fetch(zips[0]["browser_download_url"], args.cache, zips[0]["name"])
            install_zip(path)
        else:
            path = fetch(rel["zipball_url"], args.cache, f"EdgeFlow-{rel['tag_name']}.zip")
            install_folder_zip(path, module)
        log("SUCCESS", name, f"installed {rel['tag_name']}")
    except Exception as e:  # noqa: BLE001
        log("FAILED", name, f"download or install: {e}")
        return
    enable(name, module, args.dry_run)


def blendergis(args):
    """BlenderGIS from its latest GitHub release (source zip; the repository has no packaged zip)."""
    name, module = "BlenderGIS", "BlenderGIS"
    if module in module_names():
        enable(name, module, args.dry_run)
        return
    if args.dry_run:
        log("SKIPPED", name, "would install the latest GitHub release (dry run)")
        return
    if not online(name):
        return
    try:
        rel = github_latest("domlysz/BlenderGIS")
        path = fetch(rel["zipball_url"], args.cache, f"BlenderGIS-{rel['tag_name']}.zip")
        install_folder_zip(path, module)
        addon_utils.modules(refresh=True)
        log("SUCCESS", name, f"installed release {rel['tag_name']} (GDAL is optional: without it, raster import is limited)")
    except Exception as e:  # noqa: BLE001
        log("FAILED", name, f"download or install: {e}")
        return
    enable(name, module, args.dry_run)


def blenderkit(args):
    name = "BlenderKit"
    if not args.include_interactive:
        log("SKIPPED", name, "UI only (login, asset browser and its background client); pass --include-interactive to install it anyway")
        return
    installed = [m for m in module_names() if m.split(".")[-1] == "blenderkit"]
    if installed:
        enable(name, installed[0], args.dry_run)
        return
    if args.dry_run:
        log("SKIPPED", name, "would install the latest GitHub release (dry run)")
        return
    if not online(name):
        return
    try:
        rel = github_latest("BlenderKit/BlenderKit")
        asset = next(a for a in rel["assets"] if a["name"].endswith(".zip"))
        path = fetch(asset["browser_download_url"], args.cache, asset["name"])
        install_zip(path)
        log("SUCCESS", name, f"installed {rel['tag_name']}")
    except Exception as e:  # noqa: BLE001
        log("FAILED", name, f"download or install: {e}")
        return
    found = [m for m in module_names() if m.split(".")[-1] == "blenderkit"]
    if found:
        enable(name, found[0], args.dry_run)
    else:
        log("FAILED", name, "installed, but its module was not found")


def machin3tools(args):
    name = "MACHIN3tools"
    found = [m for m in module_names() if m.split(".")[-1].lower() == "machin3tools"]
    if found:
        enable(name, found[0], args.dry_run)
        return
    if not args.machin3_zip:
        log("SKIPPED", name, "no verified public download (get it from machin3.io or Superhive, then pass --machin3-zip PATH); UI only")
        return
    if not os.path.isfile(args.machin3_zip):
        log("FAILED", name, f"no file at {args.machin3_zip}")
        return
    if args.dry_run:
        log("SKIPPED", name, f"would install {args.machin3_zip} (dry run)")
        return
    try:
        install_zip(args.machin3_zip)
        log("SUCCESS", name, f"installed {os.path.basename(args.machin3_zip)}")
    except Exception as e:  # noqa: BLE001
        log("FAILED", name, f"install: {e}")
        return
    found = [m for m in module_names() if m.split(".")[-1].lower() == "machin3tools"]
    if found:
        enable(name, found[0], args.dry_run)


# ---------------------------------------------------------------- the list

def main():
    a = args()
    print(f"Blender {bpy.app.version_string} ({'Extensions Platform' if MODERN else 'legacy add-ons'}); "
          f"online access {'on' if bpy.app.online_access else 'off'}; user add-ons in {user_addons_dir()}", flush=True)
    if MODERN:
        bundled("Node Wrangler", "node_wrangler", a.dry_run)
        bundled("Rigify", "rigify", a.dry_run)
        for name, pkg in [
            ("LoopTools", "looptools"),
            ("Extra Mesh Objects", "extra_mesh_objects"),
            # (Its id on extensions.blender.org is spelt so.)
            ("Extra Curve Objects", "extra_curve_objectes"),
            ("Auto Mirror", "auto_mirror"),
            ("F2", "f2"),
            ("EdgeFlow", "EdgeFlow"),
        ]:
            extension(name, pkg, a.dry_run)
    else:
        for name, module in [
            ("Node Wrangler", "node_wrangler"),
            ("LoopTools", "mesh_looptools"),
            ("Rigify", "rigify"),
            ("Extra Mesh Objects", "add_mesh_extra_objects"),
            ("Extra Curve Objects", "add_curve_extra_objects"),
            ("Auto Mirror", "mesh_auto_mirror"),
            ("F2", "mesh_f2"),
        ]:
            bundled(name, module, a.dry_run)
        edgeflow_legacy(a)
    blendergis(a)
    blenderkit(a)
    machin3tools(a)

    if not a.dry_run:
        try:
            bpy.ops.wm.save_userpref()
            log("SUCCESS", "Preferences", "saved")
        except Exception as e:  # noqa: BLE001
            log("FAILED", "Preferences", f"not saved: {e}")

    failed = [r for r in RESULTS if r[0] == "FAILED"]
    print(f"\nDone: {sum(r[0] == 'SUCCESS' for r in RESULTS)} succeeded, "
          f"{sum(r[0] == 'SKIPPED' for r in RESULTS)} skipped, {len(failed)} failed.", flush=True)
    if failed and bpy.app.background:
        sys.exit(1)


if __name__ == "__main__":
    main()
