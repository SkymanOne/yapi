#!/bin/sh
# Packages a release build as installers expect it: yapi-<target>.tar.gz
# holding the yapi binary and the chest binary, and
# yapi-<target>.tar.gz.sha256 in sha256sum's format. The archive also holds
# LICENSE-MIT, LICENSE-APACHE and <notices>, which
# scripts/third-party-notices.sh writes, as THIRD-PARTY-NOTICES. The names
# carry no version, so the latest release's archive is always at
# releases/latest/download/yapi-<target>.tar.gz. install.sh and the
# cargo-binstall metadata in crates/yapi/Cargo.toml and crates/chest/Cargo.toml
# rely on these names. cargo-binstall takes one binary from the archive: yapi
# for the yapi crate, and chest for chest-cli, whose metadata names the
# release's tag literally because chest has its own version. Releases build both
# binaries, and scripts/check-release.sh requires both. Without <chest>, the
# archive holds yapi alone, as archives did before chest shipped.
#
# Usage: scripts/package-release.sh <yapi> <target> <out-dir> <notices> [<chest>]
set -eu

if [ $# -ne 4 ] && [ $# -ne 5 ]; then
    echo "Usage: $0 <yapi> <target> <out-dir> <notices> [<chest>]" >&2
    exit 1
fi
binary=$1
chest=${5:-}
name="yapi-$2"
root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$3"
out=$(cd "$3" && pwd)

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$binary" "$work/yapi"
chmod 755 "$work/yapi"
binaries=yapi
if [ -n "$chest" ]; then
    cp "$chest" "$work/chest"
    chmod 755 "$work/chest"
    binaries="yapi chest"
fi
cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$work/"
cp "$4" "$work/THIRD-PARTY-NOTICES"
# macOS tar would otherwise add AppleDouble files for extended attributes.
# shellcheck disable=SC2086 # $binaries is a list of names without spaces.
(cd "$work" && COPYFILE_DISABLE=1 tar -czf "$out/$name.tar.gz" $binaries LICENSE-MIT LICENSE-APACHE THIRD-PARTY-NOTICES)

cd "$out"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$name.tar.gz" >"$name.tar.gz.sha256"
else
    shasum -a 256 "$name.tar.gz" >"$name.tar.gz.sha256"
fi
echo "$out/$name.tar.gz"
