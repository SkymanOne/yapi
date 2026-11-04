#!/bin/sh
# Installs yapi from a GitHub release: downloads the archive for this system,
# checks it against its published SHA-256 and copies yapi, and chest when the
# release has it, into a directory on PATH. Then it installs the release's
# docs for the model the same way into yapi's agent directory, replacing the
# previous copy. It can install yapi or chest alone instead. Without that
# choice, it asks when it runs in a terminal and installs both otherwise.
#
#   curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh -s -- --version v0.3.1 --to /usr/local/bin
#   curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh -s -- --only chest
#
# Options, or the environment variables that set them:
#   --version <tag>   YAPI_VERSION       release to install (default: the latest)
#   --to <dir>        YAPI_INSTALL_DIR   where to put yapi and chest (default: ~/.local/bin)
#   --only <name>     YAPI_INSTALL_ONLY  yapi, or chest without the docs, or both
#                                        (default: ask in a terminal, else both)
#   --no-docs         YAPI_NO_DOCS=1     skip the docs for the model
#   YAPI_CODING_AGENT_DIR                where the docs go, under docs/
#                                        (default: ~/.yapi/agent)
#   YAPI_RELEASES_URL                    the releases page (default: GitHub's)
set -eu

releases="${YAPI_RELEASES_URL:-https://github.com/SkymanOne/yapi/releases}"
releases="${releases%/}"
version="${YAPI_VERSION:-latest}"
dir="${YAPI_INSTALL_DIR:-}"
case "${YAPI_NO_DOCS:-}" in
    1 | true | TRUE | True | yes | YES | Yes) docs= ;;
    *) docs=yes ;;
esac
only="${YAPI_INSTALL_ONLY:-}"

fail() {
    echo "install.sh: $*" >&2
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            [ $# -ge 2 ] || fail "--version needs a value"
            version=$2
            shift 2
            ;;
        --to)
            [ $# -ge 2 ] || fail "--to needs a value"
            dir=$2
            shift 2
            ;;
        --only)
            [ $# -ge 2 ] || fail "--only needs a value"
            only=$2
            shift 2
            ;;
        --no-docs)
            docs=
            shift
            ;;
        -h | --help)
            cat <<EOF
Usage: install.sh [--version <tag>] [--to <dir>] [--only yapi|chest|both] [--no-docs]

Installs yapi and chest from a GitHub release, checked against its SHA-256,
and the release's docs for the model into ~/.yapi/agent/docs, or
YAPI_CODING_AGENT_DIR. Without --only, it asks in a terminal whether to
install both or one.

  --version <tag>   release to install (default: the latest), or YAPI_VERSION
  --to <dir>        where to put yapi and chest (default: ~/.local/bin), or YAPI_INSTALL_DIR
  --only <name>     yapi, or chest without the docs, or both, or YAPI_INSTALL_ONLY
  --no-docs         skip the docs, or YAPI_NO_DOCS=1
EOF
            exit 0
            ;;
        *) fail "unknown option $1" ;;
    esac
done
if [ -z "$dir" ]; then
    [ -n "${HOME:-}" ] || fail "HOME is not set. Choose where to install yapi with --to <dir> or YAPI_INSTALL_DIR."
    dir="$HOME/.local/bin"
fi
case "$only" in
    '' | yapi | chest | both) ;;
    *) fail "--only and YAPI_INSTALL_ONLY take yapi, chest or both, not '$only'" ;;
esac

# Asks only when someone can answer: stdout is a terminal and /dev/tty opens.
# With curl | sh, stdin holds the script, so the answer comes from /dev/tty.
if [ -z "$only" ] && [ -t 1 ] && (: </dev/tty) 2>/dev/null; then
    printf 'Install:\n  1) yapi and chest\n  2) yapi only\n  3) chest only\n' >/dev/tty
    tries=0
    while [ -z "$only" ]; do
        printf 'Choose 1, 2 or 3 [1]: ' >/dev/tty
        read -r answer </dev/tty || fail "no answer. Run the script again, or pass --only yapi, chest or both."
        case "$answer" in
            '' | 1) only=both ;;
            2) only=yapi ;;
            3) only=chest ;;
            *)
                tries=$((tries + 1))
                [ "$tries" -lt 3 ] || fail "no valid choice. Run the script again, or pass --only yapi, chest or both."
                ;;
        esac
    done
fi
case "$only" in
    yapi) binaries=yapi crates=yapi ;;
    # The docs are for yapi's model.
    chest) binaries=chest crates=chest-cli docs= ;;
    *) binaries="yapi chest" crates="yapi chest-cli" ;;
esac
from_source="cargo install --locked --git https://github.com/SkymanOne/yapi $crates"

command -v curl >/dev/null 2>&1 || fail "needs curl"
command -v tar >/dev/null 2>&1 || fail "needs tar"
if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
    fail "needs sha256sum or shasum to check the download"
fi

# The release target for this system.
os=$(uname -s)
arch=$(uname -m)
case "$arch" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *) fail "no release build for $arch. Build from source: $from_source" ;;
esac
case "$os" in
    Linux)
        if ldd --version 2>&1 | grep -qi musl; then
            fail "release builds need glibc, and this system uses musl. Build from source: $from_source"
        fi
        glibc=$(getconf GNU_LIBC_VERSION 2>/dev/null | sed 's/^glibc //')
        case "$glibc" in
            2.[0-9] | 2.[0-9].* | 2.[12][0-9] | 2.[12][0-9].* | 2.3[0-4] | 2.3[0-4].*)
                fail "release builds need glibc 2.35 or newer, and this system has $glibc. Build from source: $from_source"
                ;;
        esac
        target="$arch-unknown-linux-gnu"
        ;;
    Darwin)
        # A shell under Rosetta reports x86_64 on Apple silicon.
        if [ "$arch" = x86_64 ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = 1 ]; then
            arch=aarch64
        fi
        target="$arch-apple-darwin"
        ;;
    *) fail "no release build for $os. Build from source: $from_source" ;;
esac

# Release archives have no version in their names, so the latest release's
# archive has a fixed URL.
if [ "$version" = latest ]; then
    download="$releases/latest/download"
    release="the latest release"
else
    download="$releases/download/v${version#v}"
    release="release v${version#v}"
fi

name="yapi-$target.tar.gz"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Downloads file $1 of the release into $work and checks it against its
# published SHA-256. On failure, $problem says why.
fetch_checked() {
    if ! curl -fsSL -o "$work/$1" "$download/$1"; then
        problem="found no $1 in $release at $releases"
    elif ! curl -fsSL -o "$work/$1.sha256" "$download/$1.sha256"; then
        problem="could not download the checksum for $1"
    elif [ "$(cut -d ' ' -f 1 <"$work/$1.sha256")" != "$(sha256 "$work/$1")" ]; then
        problem="$1 does not match its SHA-256 checksum"
    else
        return 0
    fi
    return 1
}

echo "Downloading $name from $release"
fetch_checked "$name" || fail "$problem. Build from source: $from_source"

mkdir "$work/unpacked"
tar -xzf "$work/$name" -C "$work/unpacked"
if [ "$only" != chest ]; then
    [ -f "$work/unpacked/yapi" ] || fail "$name does not contain yapi"
    installed=$("$work/unpacked/yapi" --version) || fail "the downloaded yapi does not run on this system"
fi
if [ "$only" = yapi ]; then
    :
elif [ -f "$work/unpacked/chest" ]; then
    chest=$("$work/unpacked/chest" --version) || fail "the downloaded chest does not run on this system"
elif [ "$only" = chest ]; then
    fail "$name holds no chest, because releases before chest shipped hold yapi alone. Build chest from source: $from_source"
else
    # Releases before chest shipped hold yapi alone.
    binaries=yapi
fi
mkdir -p "$dir"
for binary in $binaries; do
    # Copy, then rename, so a running binary is replaced rather than overwritten.
    cp "$work/unpacked/$binary" "$dir/.$binary.$$"
    chmod 755 "$dir/.$binary.$$"
    mv -f "$dir/.$binary.$$" "$dir/$binary"
done
[ "$binaries" = chest ] || echo "Installed yapi $installed to $dir/yapi"
[ "$binaries" = yapi ] || echo "Installed $chest to $dir/chest"

# The docs the model reads about yapi and pi, unpacked next to the old copy,
# which they then replace. yapi downloads them on its first run when this
# step fails.
if [ -n "$docs" ]; then
    agent="${YAPI_CODING_AGENT_DIR:-}"
    # shellcheck disable=SC2088 # A leading ~/ the shell did not expand.
    case "$agent" in
        "~/"*) agent="${HOME:-}/${agent#\~/}" ;;
    esac
    [ -n "$agent" ] || agent="${HOME:+$HOME/.yapi/agent}"
    if [ -z "$agent" ]; then
        problem="HOME and YAPI_CODING_AGENT_DIR are not set"
    elif ! fetch_checked yapi-docs.tar.gz; then
        : # fetch_checked set $problem.
    elif ! mkdir -p "$agent/.docs.$$" || ! tar -xzf "$work/yapi-docs.tar.gz" -C "$agent/.docs.$$"; then
        rm -rf "$agent/.docs.$$"
        problem="could not unpack yapi-docs.tar.gz into $agent"
    elif ! rm -rf "$agent/docs" || ! mv "$agent/.docs.$$" "$agent/docs"; then
        rm -rf "$agent/.docs.$$"
        problem="could not move the docs into $agent/docs"
    else
        problem=
        echo "Installed the docs for the model to $agent/docs"
    fi
    if [ -n "$problem" ]; then
        echo "install.sh: skipped the docs for the model: $problem. yapi downloads them when it first runs." >&2
    fi
fi

[ "$binaries" != "yapi chest" ] || binaries="yapi and chest"
case ":${PATH:-}:" in
    *":$dir:"*) ;;
    *) echo "Add $dir to PATH to run $binaries, for example in your shell's profile: export PATH=\"$dir:\$PATH\"" ;;
esac
