#!/bin/sh
# Install a checksummed Proqi release for x86-64 Linux without root.
#
#   curl -fsSL https://raw.githubusercontent.com/oborchers/proqi/main/scripts/install.sh | sh
#
# The script downloads one published release archive, verifies its SHA-256
# checksum, proves the executable runs on this host, and only then installs it.
# It never runs sudo, a package manager, or any command on the user's behalf.

set -eu

RELEASES_URL="${PROQI_RELEASES_URL:-https://github.com/oborchers/proqi/releases}"
PACKAGE="proqi-x86_64-unknown-linux-gnu"
SOURCE_FALLBACK="Build from source instead: cargo install proqi --locked"

prefix="${PROQI_PREFIX:-}"
version="${PROQI_VERSION:-}"
checksum_tool=""
libc="unknown"
libc_report=""
work=""
archive=""
staged=""
bin=""
previous=""
released=""

main() {
    parse_arguments "$@"
    require_supported_host
    require_tools
    resolve_version
    create_work_directory
    download_release
    verify_checksum
    extract_release
    verify_executable
    install_release
    report
}

usage() {
    cat <<'EOF'
Install Proqi on x86-64 Linux from a checksummed GitHub Release archive.

Usage:
  install.sh [--prefix DIR] [--version VERSION]

Options:
  --prefix DIR       Installation prefix. The executable is placed in DIR/bin.
                     Default: $HOME/.local
  --version VERSION  Release to install, such as 0.8.0 or v0.8.0.
                     Default: the latest published release.
  --help             Print this message and exit.

Environment:
  PROQI_PREFIX        Same as --prefix.
  PROQI_VERSION       Same as --version.
  PROQI_RELEASES_URL  Base URL of the release store.
                      Default: https://github.com/oborchers/proqi/releases

Re-running this script installs the requested release over the previous one.
Uninstall by deleting DIR/bin/proqi and DIR/bin/proqi-installation.json;
boards and settings are kept.
EOF
}

fail() {
    printf 'proqi install: %s\n' "$1" >&2
    shift
    for line in "$@"; do
        printf '  %s\n' "$line" >&2
    done
    exit 1
}

parse_arguments() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
        --prefix)
            [ "$#" -gt 1 ] || fail '--prefix needs a directory.'
            prefix=$2
            shift
            ;;
        --prefix=*) prefix=${1#--prefix=} ;;
        --version)
            [ "$#" -gt 1 ] || fail '--version needs a release.'
            version=$2
            shift
            ;;
        --version=*) version=${1#--version=} ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) fail "unknown option: $1" 'Run install.sh --help for supported options.' ;;
        esac
        shift
    done
    [ -n "$prefix" ] || prefix=$(default_prefix)
    [ -n "$prefix" ] || fail 'no installation prefix is known.' \
        'Set HOME, or choose one with --prefix DIR.'
}

default_prefix() {
    if [ -n "${HOME:-}" ]; then
        printf '%s' "$HOME/.local"
    fi
}

require_supported_host() {
    system=$(uname -s 2>/dev/null || echo unknown)
    [ "$system" = Linux ] || unsupported_system "$system"
    machine=$(uname -m 2>/dev/null || echo unknown)
    case "$machine" in
    x86_64 | amd64) ;;
    *)
        fail "$machine Linux has no published Proqi release." \
            'Releases are built for one Linux target: x86_64 with glibc.' \
            "$SOURCE_FALLBACK"
        ;;
    esac
    detect_libc
    [ "$libc" != musl ] || fail \
        'a musl C library cannot run the published Proqi release.' \
        "The $PACKAGE archive links against glibc." \
        "Detected: ${libc_report:-musl}" \
        "$SOURCE_FALLBACK"
}

unsupported_system() {
    case "$1" in
    Darwin)
        fail "$1 is not installed by this script." \
            'Install Proqi on macOS with Homebrew:' \
            'brew install oborchers/tap/proqi'
        ;;
    *)
        fail "$1 is not a supported Proqi platform." \
            'This script installs x86-64 Linux releases only.'
        ;;
    esac
}

detect_libc() {
    if [ -f /etc/alpine-release ]; then
        libc=musl
        libc_report='Alpine Linux'
        return 0
    fi
    libc_report=$(ldd --version 2>&1 | head -n 1)
    case "$libc_report" in
    *musl*) libc=musl ;;
    *'GNU libc'* | *GLIBC* | *glibc*) libc=glibc ;;
    esac
}

require_tools() {
    for tool in curl tar mktemp; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "$tool is required to install Proqi but was not found."
    done
    for tool in sha256sum shasum openssl; do
        if command -v "$tool" >/dev/null 2>&1; then
            checksum_tool=$tool
            break
        fi
    done
    [ -n "$checksum_tool" ] || fail \
        'no SHA-256 tool was found (sha256sum, shasum, or openssl).' \
        'Proqi is never installed without verifying its published checksum.'
}

compute_sha256() {
    case "$checksum_tool" in
    sha256sum) sha256sum "$1" | cut -d ' ' -f 1 ;;
    shasum) shasum -a 256 "$1" | cut -d ' ' -f 1 ;;
    openssl) openssl dgst -sha256 "$1" | tr ' ' '\n' | tail -n 1 ;;
    esac
}

resolve_version() {
    [ -n "$version" ] || resolve_latest_version
    case "$version" in
    v*) ;;
    *) version="v$version" ;;
    esac
    case "$version" in
    v[0-9]*) ;;
    *) fail "not a Proqi release version: $version" ;;
    esac
}

resolve_latest_version() {
    located=$(curl -fsSL --retry 2 -o /dev/null -w '%{url_effective}' "$RELEASES_URL/latest") ||
        located=""
    version=${located##*/}
    case "$version" in
    v[0-9]*) ;;
    *) fail "could not resolve the latest Proqi release from $RELEASES_URL/latest." \
        'Pass an explicit release with --version, such as --version 0.8.0.' ;;
    esac
}

create_work_directory() {
    work=$(mktemp -d "${TMPDIR:-/tmp}/proqi-install.XXXXXX") || work=""
    [ -n "$work" ] || fail 'could not create a temporary directory.'
    trap cleanup EXIT
    trap 'cleanup; exit 130' INT
    trap 'cleanup; exit 129' HUP
    trap 'cleanup; exit 143' TERM
}

cleanup() {
    [ -z "$work" ] || rm -rf "$work"
    work=""
}

download_release() {
    archive="$work/$PACKAGE.tar.gz"
    fetch "$RELEASES_URL/download/$version/$PACKAGE.tar.gz" "$archive"
    fetch "$RELEASES_URL/download/$version/$PACKAGE.tar.gz.sha256" "$archive.sha256"
}

fetch() {
    curl -fsSL --retry 2 -o "$2" "$1" || fail "could not download $1" \
        "Check that release $version exists and that the network is reachable."
}

verify_checksum() {
    published=$(cut -d ' ' -f 1 <"$archive.sha256") || published=""
    observed=$(compute_sha256 "$archive") || observed=""
    [ -n "$published" ] || fail "the published checksum for $version is unreadable."
    [ "$published" = "$observed" ] || fail \
        "checksum mismatch for $PACKAGE.tar.gz" \
        "published:  $published" \
        "downloaded: $observed" \
        'Nothing was installed.'
}

extract_release() {
    tar -xzf "$archive" -C "$work" || fail "could not extract $PACKAGE.tar.gz."
    staged="$work/$PACKAGE"
    for member in proqi proqi-installation.json; do
        [ -f "$staged/$member" ] ||
            fail "the $version archive is missing $member."
    done
}

verify_executable() {
    chmod 755 "$staged/proqi" || fail 'could not make the downloaded executable runnable.'
    reported=$("$staged/proqi" --version 2>/dev/null) || reported=""
    [ -n "$reported" ] || fail \
        'the downloaded proqi executable did not run on this host.' \
        "Detected C library: ${libc_report:-unknown}" \
        'Linux releases need glibc 2.35 or newer. Nothing was installed.' \
        "$SOURCE_FALLBACK"
    released=${reported#proqi }
    [ "$released" = "${version#v}" ] || fail \
        "the downloaded executable reports $released, not ${version#v}." \
        'Nothing was installed.'
}

install_release() {
    bin="$prefix/bin"
    mkdir -p "$bin" || fail "could not create $bin."
    [ -w "$bin" ] || fail "$bin is not writable." \
        'Choose a writable location with --prefix DIR.'
    previous=$(reported_version "$bin/proqi")
    # The executable lands last so an interrupted run leaves the previous one.
    stage "$staged/proqi-installation.json" "$bin/proqi-installation.json" 644
    stage "$staged/proqi" "$bin/proqi" 755
}

reported_version() {
    [ -x "$1" ] || return 0
    output=$("$1" --version 2>/dev/null) || return 0
    printf '%s' "${output#proqi }"
}

stage() {
    pending="$2.pending.$$"
    if ! cp "$1" "$pending" || ! chmod "$3" "$pending"; then
        rm -f "$pending"
        fail "could not stage $2."
    fi
    mv -f "$pending" "$2" || {
        rm -f "$pending"
        fail "could not install $2."
    }
}

report() {
    if [ -z "$previous" ]; then
        printf 'Installed proqi %s to %s\n' "$released" "$bin/proqi"
    elif [ "$previous" = "$released" ]; then
        printf 'Reinstalled proqi %s to %s\n' "$released" "$bin/proqi"
    else
        printf 'Replaced proqi %s with %s at %s\n' "$previous" "$released" "$bin/proqi"
    fi
    report_path
}

report_path() {
    case ":${PATH:-}:" in
    *":$bin:"*) return 0 ;;
    esac
    printf '\n%s is not on your PATH. Add it to your shell profile with:\n\n' "$bin"
    printf '  export PATH="%s:$PATH"\n' "$bin"
}

main "$@"
