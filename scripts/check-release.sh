#!/bin/sh
# Checks that every archive in <dist-dir> carries yapi, chest and the
# licenses, and that yapi-docs.tar.gz holds yapi's and pi's docs, then
# installs this system's release archive as users would: with install.sh, as
# the latest release and as release <tag>, and with cargo-binstall when it is
# on PATH. install.sh must install yapi, chest and the docs, and
# cargo-binstall yapi alone for the yapi crate and chest alone for chest-cli,
# both from the archive. chest has its own version, from crates/chest.
# The archives are served over HTTPS from 127.0.0.1 in the layout of GitHub's
# release downloads, with a throwaway certificate authority.
#
# Usage: scripts/check-release.sh <dist-dir> <tag>
# Needs curl, openssl and python3.
set -eu

if [ $# -ne 2 ]; then
    echo "Usage: $0 <dist-dir> <tag>" >&2
    exit 1
fi
dist=$(cd "$1" && pwd)
tag=$2
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
server=
cleanup() {
    [ -z "$server" ] || kill "$server" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

for archive in "$dist"/yapi-*.tar.gz; do
    [ "$archive" != "$dist/yapi-docs.tar.gz" ] || continue
    files=$(tar -tzf "$archive" | LC_ALL=C sort | tr '\n' ' ')
    if [ "$files" != "LICENSE-APACHE LICENSE-MIT THIRD-PARTY-NOTICES chest yapi " ]; then
        echo "$archive holds $files" >&2
        exit 1
    fi
done
echo "ok: every archive holds yapi, chest and their licenses"
docs=$(tar -tzf "$dist/yapi-docs.tar.gz")
for file in .version index.md compat.md pi/LICENSE pi/README.md pi/docs/extensions.md pi/examples/extensions/hello.ts; do
    if ! printf '%s\n' "$docs" | grep -qx "$file"; then
        echo "yapi-docs.tar.gz lacks $file" >&2
        exit 1
    fi
done
if printf '%s\n' "$docs" | grep -q '^pi/examples/plugins/'; then
    echo "yapi-docs.tar.gz holds pi's plugin example" >&2
    exit 1
fi
echo "ok: the docs archive holds yapi's and pi's docs"

# Without HOME, install.sh has no default directory and must not pick /.local/bin.
if message=$(env -u HOME YAPI_RELEASES_URL=https://127.0.0.1:9 sh "$root/install.sh" 2>&1); then
    echo "install.sh without HOME succeeded: $message" >&2
    exit 1
fi
case "$message" in
    *"HOME is not set"*) echo "ok: install.sh without HOME asks for a directory" ;;
    *)
        echo "install.sh without HOME failed with: $message" >&2
        exit 1
        ;;
esac

# GitHub serves the latest release's files under latest/download as well.
for path in "download/$tag" latest/download; do
    mkdir -p "$work/site/SkymanOne/yapi/releases/$path"
    cp "$dist"/yapi-* "$work/site/SkymanOne/yapi/releases/$path/"
done

# A certificate authority and a certificate for 127.0.0.1 that it signs. The
# extensions are explicit: macOS's verifier, which cargo-binstall uses there,
# accepts a server certificate only for the serverAuth key usage.
cd "$work"
cat >certs.cnf <<'EOF'
[req]
distinguished_name = name
[name]
[ca]
basicConstraints = critical, CA:TRUE
keyUsage = critical, keyCertSign, cRLSign
[server]
basicConstraints = critical, CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = IP:127.0.0.1
EOF
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem -days 1 \
    -subj "/CN=yapi release check" -config certs.cnf -extensions ca 2>/dev/null
openssl req -newkey rsa:2048 -nodes -keyout key.pem -out server.csr \
    -subj "/CN=127.0.0.1" -config certs.cnf 2>/dev/null
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
    -out cert.pem -days 1 -extfile certs.cnf -extensions server 2>/dev/null

cat >serve.py <<'EOF'
import functools, http.server, socketserver, ssl


class Server(http.server.ThreadingHTTPServer):
    # HTTPServer.server_bind looks up the address's host name, which can take
    # many seconds on macOS. Nothing here uses the name.
    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        self.server_name, self.server_port = self.server_address[:2]


handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory="site")
httpd = Server(("127.0.0.1", 0), handler)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain("cert.pem", "key.pem")
httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
print(httpd.server_address[1], flush=True)
httpd.serve_forever()
EOF
python3 serve.py >port 2>server.log &
server=$!
waited=0
while [ ! -s port ] && [ "$waited" -lt 60 ]; do
    sleep 1
    waited=$((waited + 1))
done
if [ ! -s port ]; then
    echo "The HTTPS server did not start within 60 s. $(python3 --version 2>&1), log:" >&2
    cat server.log >&2
    exit 1
fi
repo="https://127.0.0.1:$(cat port)/SkymanOne/yapi"
version=${tag#v}
chest_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/crates/chest/Cargo.toml")

# Checks that installer $2 put only the binaries named after it, in name
# order and of this release's versions, into directory $1.
check() {
    dir=$1
    installer=$2
    shift 2
    installed=$(ls -A "$dir")
    if [ "$installed" != "$(printf '%s\n' "$@")" ]; then
        echo "$installer installed $installed" >&2
        exit 1
    fi
    for binary in "$@"; do
        # chest prints its name before its own version.
        expected=$version
        [ "$binary" = yapi ] || expected="chest $chest_version"
        found=$("$dir/$binary" --version)
        if [ "$found" != "$expected" ]; then
            echo "$dir/$binary --version printed '$found', not '$expected'" >&2
            exit 1
        fi
    done
    echo "ok: $installer installed $* of release $version"
}

# The docs install.sh unpacked into agent directory $1.
check_docs() {
    if [ "$(cat "$1/docs/.version" 2>/dev/null)" != "$version" ] || [ ! -f "$1/docs/pi/docs/extensions.md" ]; then
        echo "$2 did not install the docs for $version into $1/docs" >&2
        exit 1
    fi
    echo "ok: $2 installed the docs"
}

CURL_CA_BUNDLE="$work/ca.pem" YAPI_RELEASES_URL="$repo/releases" YAPI_CODING_AGENT_DIR="$work/latest/agent" \
    sh "$root/install.sh" --to "$work/latest/bin"
check "$work/latest/bin" "install.sh, latest release," chest yapi
check_docs "$work/latest/agent" "install.sh, latest release,"
CURL_CA_BUNDLE="$work/ca.pem" YAPI_RELEASES_URL="$repo/releases" YAPI_CODING_AGENT_DIR="$work/tagged/agent" \
    sh "$root/install.sh" --version "$tag" --to "$work/tagged/bin"
check "$work/tagged/bin" "install.sh, release $tag," chest yapi
check_docs "$work/tagged/agent" "install.sh, release $tag,"

if command -v cargo-binstall >/dev/null 2>&1; then
    # The repository's manifests, pointing at this server instead of GitHub.
    # Only the archive counts, not a build from crates.io.
    manifest="$work/manifest"
    mkdir -p "$manifest/crates/yapi/src" "$manifest/crates/chest/src"
    sed "s|^repository = .*|repository = \"$repo\"|" "$root/Cargo.toml" >"$manifest/Cargo.toml"
    cp "$root/crates/yapi/Cargo.toml" "$manifest/crates/yapi/"
    cp "$root/crates/chest/Cargo.toml" "$manifest/crates/chest/"
    : >"$manifest/crates/yapi/src/main.rs"
    : >"$manifest/crates/chest/src/main.rs"
    for crate in yapi chest-cli; do
        cargo-binstall --manifest-path "$manifest" "$crate" --no-confirm --disable-telemetry \
            --strategies crate-meta-data --root "$work/binstall-$crate" --root-certificates "$work/ca.pem"
    done
    check "$work/binstall-yapi/bin" "cargo-binstall yapi" yapi
    check "$work/binstall-chest-cli/bin" "cargo-binstall chest-cli" chest
else
    echo "skipped: cargo-binstall is not on PATH"
fi
