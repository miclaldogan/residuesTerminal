#!/usr/bin/env bash
#
#   R E S I D U E S  —  zero-install launcher
#
#   curl -sSL https://raw.githubusercontent.com/miclaldogan/residuesTerminal/main/run.sh | bash
#
# Downloads the prebuilt binary + its audio/ and images/ assets into a throwaway
# temp directory, runs the game, then wipes every trace on exit. POSIX-sh clean —
# no bashisms — so it behaves identically whether the pipe lands in bash, dash, or
# zsh, on Linux or macOS.

set -eu

# ── Configuration ───────────────────────────────────────────────────────────
REPO="miclaldogan/residuesTerminal"
BIN_NAME="residues_terminal"
# Pin a tag with:  RESIDUES_VERSION=v1.0.0 curl -sSL …/run.sh | bash
VERSION="${RESIDUES_VERSION:-latest}"

# ── Atmospheric output (amber, only when attached to a real terminal) ────────
if [ -t 1 ]; then
    AMBER="$(printf '\033[38;5;179m')"
    DIM="$(printf '\033[38;5;94m')"
    RST="$(printf '\033[0m')"
else
    AMBER=""
    DIM=""
    RST=""
fi

say()  { printf '%s%s%s\n' "$AMBER" "$1" "$RST"; }
hush() { printf '%s%s%s\n' "$DIM" "$1" "$RST"; }
die()  { printf 'residues: %s\n' "$1" >&2; exit 1; }

# A short typewriter ellipsis to fill the synchronisation beat.
ellipsis() {
    _n=0
    while [ "$_n" -lt 4 ]; do
        printf '%s . %s' "$AMBER" "$RST"
        sleep 0.2 2>/dev/null || sleep 1
        _n=$((_n + 1))
    done
    printf '\n'
}

say "[SYSTEM]: Synchronizing residual mind matrices... Please standby"
ellipsis

# ── Platform detection (uname -s / uname -m) ────────────────────────────────
os="$(uname -s 2>/dev/null || echo unknown)"
arch="$(uname -m 2>/dev/null || echo unknown)"

case "$os" in
    Linux)  OS="linux"  ;;
    Darwin) OS="macos"  ;;
    *) die "unsupported operating system '$os' (Linux and macOS only)" ;;
esac

case "$arch" in
    x86_64 | amd64) ARCH="x86_64"  ;;
    arm64)          ARCH="arm64"   ;;
    aarch64)        ARCH="aarch64" ;;
    *) die "unsupported architecture '$arch'" ;;
esac

ASSET="${BIN_NAME}-${OS}-${ARCH}.tar.gz"
if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi
hush "  · target detected: ${OS}/${ARCH}"

# ── Downloader selection (curl or wget) ─────────────────────────────────────
if command -v curl >/dev/null 2>&1; then
    DL="curl"
elif command -v wget >/dev/null 2>&1; then
    DL="wget"
else
    die "need either 'curl' or 'wget' on PATH to fetch the package"
fi

# ── Isolated temp workspace + self-destruct trap ────────────────────────────
# Everything we download lives here and nowhere else; the trap guarantees the
# directory (binary + assets) is erased the instant the game closes or the
# script is interrupted — the user's home stays pristine.
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/residues_launch.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

PKG="${TMP_DIR}/package.tar.gz"

# ── Retrieve the consolidated tarball (binary + audio/ + images/) ───────────
hush "  · retrieving payload via ${DL}"
if [ "$DL" = "curl" ]; then
    curl -fsSL "$URL" -o "$PKG" || die "download failed from ${URL}"
else
    wget -q "$URL" -O "$PKG" || die "download failed from ${URL}"
fi
[ -s "$PKG" ] || die "downloaded package is empty (no release asset '${ASSET}'?)"

# ── Extract silently inside the temp bounds ─────────────────────────────────
hush "  · decompressing assets into volatile memory"
tar -xzf "$PKG" -C "$TMP_DIR" || die "could not extract package"

# Locate the binary wherever it landed (top-level or one folder deep) so the
# AudioEngine / ImageEngine find the sibling audio/ and images/ directories.
BIN_PATH="$(find "$TMP_DIR" -type f -name "$BIN_NAME" 2>/dev/null | head -n 1)"
[ -n "$BIN_PATH" ] || die "binary '${BIN_NAME}' not found inside the package"
chmod +x "$BIN_PATH" 2>/dev/null || true

# ── Launch from the binary's own directory (strictly relative paths) ────────
say "[SYSTEM]: Engine primed. Entering the mind."
RUN_DIR="$(dirname "$BIN_PATH")"
cd "$RUN_DIR"
./"$BIN_NAME"

# On return the EXIT trap fires and the whole workspace evaporates.
