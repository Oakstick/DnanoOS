#!/usr/bin/env bash
# Builds the release and puts it in a folder on the Desktop: the exe (with its full title), the
# DXC shader compiler it starts fastest with, and a note of its version. The version is the
# crate's (`Cargo.toml`), shown on the title screen and in the exe's properties; raise it before
# each release build.
#   bash tools/release.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release

name="Adventures of the Ancient Blades"
out="${USERPROFILE:-$HOME}/Desktop/$name"
rm -rf "$out"
mkdir -p "$out"
cp target/release/AdventuresOfTheAncientBlades.exe "$out/$name.exe"
for dll in dxcompiler.dll dxil.dll; do
    [ -f "target/release/$dll" ] && cp "target/release/$dll" "$out/"
done
commit=$(git rev-parse --short HEAD)
printf '%s\nVersion %s\nBuilt %s from commit %s\n' "$name" "$version" "$(date '+%Y-%m-%d %H:%M')" "$commit" > "$out/VERSION.txt"

# The repo's own copy, as before.
mkdir -p dist/windows
cp target/release/AdventuresOfTheAncientBlades.exe "dist/windows/$name.exe"

echo "Release $version ready in $out"
ls -la "$out"
