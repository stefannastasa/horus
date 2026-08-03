#!/bin/bash
set -euo pipefail

HOST=stefan-rpi.tail280288.ts.net
TARGET=aarch64-unknown-linux-gnu

BIN=horus
CTL=$(mktemp -u /tmp/cm-$BIN.XXXXXX)
BUILD_OUTPUT_DIR="/tmp/horus"
PROJECT_PATH="/Users/tefannastasa/workplace/horus"

cleanup() {
    r pkill -x horus || true
    ssh -o ControlPath="$CTL" -O exit "$HOST" 2>/dev/null || true
}

r() {
    ssh -n -o ControlPath="$CTL" "$HOST" "$@";
}

echo "Validating implementation..."
cargo test


echo "Building binary for target: $TARGET"
if ! cargo zigbuild --release --target "$TARGET" >"$BUILD_OUTPUT_DIR/build.log" 2>&1; then
    cat "$BUILD_OUTPUT_DIR/build.log"
    exit 1
fi

TARGET_LOCATION="$PROJECT_PATH/target/$TARGET/release/$BIN"
REMOTE_DIR="/tmp/horus"

ssh -o ControlMaster=yes -o ControlPath="$CTL" -o ControlPersist=no -fN "tefan@$HOST"
trap cleanup EXIT # clean the master socket after exit

r mkdir -p $REMOTE_DIR
scp -o ControlPath="$CTL" $TARGET_LOCATION tefan@"$HOST":"$REMOTE_DIR/$BIN" 2>/dev/null 1>&2

echo "Artifact transfered to remote path: $REMOTE_DIR/$BIN"

r chmod 744 "$REMOTE_DIR/$BIN"
r DOCKER_HOST=unix:///run/user/1000/podman/podman.sock exec "$REMOTE_DIR/$BIN"
