#!/bin/sh
# Install the renyi binary from the GitHub Release of a version (decision
# AI2). Usage:
#   curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh
# Environment: RENYI_VERSION (a tag such as v0.1.0; the latest release when
# unset), RENYI_INSTALL_DIR (where the binary goes; $HOME/.local/bin when
# unset), RENYI_REPO (the GitHub repository; renyi-lang/renyi when unset).
set -eu

repo="${RENYI_REPO:-renyi-lang/renyi}"
# the home directory is read at run time, never written into this file
install_dir="${RENYI_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*" >&2; }
fail() { say "install.sh: $*"; exit 1; }

need() {
  command -v "$1" > /dev/null 2>&1 || fail "needs \`$1\` on the PATH"
}
need uname
need tar
if command -v curl > /dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  fetch_text() { curl -fsSL "$1"; }
elif command -v wget > /dev/null 2>&1; then
  fetch() { wget -q "$1" -O "$2"; }
  fetch_text() { wget -q "$1" -O -; }
else
  fail "needs \`curl\` or \`wget\` on the PATH"
fi

os=$(uname -s)
arch=$(uname -m)
case "$os-$arch" in
  Linux-x86_64) target="x86_64-unknown-linux-gnu" ;;
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) fail "no binary for Intel macOS yet: \`cargo install renyi\` builds one" ;;
  *) fail "no binary for $os on $arch: \`cargo install renyi\` builds one" ;;
esac

version="${RENYI_VERSION:-}"
if [ -z "$version" ]; then
  version=$(fetch_text "https://api.github.com/repos/$repo/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)
  [ -n "$version" ] || fail "could not read the latest release of $repo"
fi

name="renyi-$version-$target"
url="https://github.com/$repo/releases/download/$version/$name.tar.gz"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

say "downloading $url"
fetch "$url" "$work/$name.tar.gz"
fetch "$url.sha256" "$work/$name.tar.gz.sha256"

if command -v sha256sum > /dev/null 2>&1; then
  (cd "$work" && sha256sum -c "$name.tar.gz.sha256" > /dev/null) || fail "the checksum does not match"
elif command -v shasum > /dev/null 2>&1; then
  (cd "$work" && shasum -a 256 -c "$name.tar.gz.sha256" > /dev/null) || fail "the checksum does not match"
else
  say "neither sha256sum nor shasum is on the PATH: the checksum was not verified"
fi

tar -C "$work" -xzf "$work/$name.tar.gz"
mkdir -p "$install_dir"
cp "$work/$name/renyi" "$install_dir/renyi"
chmod 755 "$install_dir/renyi"
say "installed $("$install_dir/renyi" --version 2> /dev/null || echo "renyi $version") to $install_dir/renyi"

case ":${PATH}:" in
  *":$install_dir:"*) ;;
  *) say "add $install_dir to your PATH, for example: export PATH=\"$install_dir:\$PATH\"" ;;
esac
