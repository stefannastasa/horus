#!/bin/bash
set -euo pipefail

MODE="${1:-dev}"

HOST=tefan@stefan-rpi.tail280288.ts.net
TARGET=aarch64-unknown-linux-gnu

BIN=horus
CTL=$(mktemp -u /tmp/cm-$BIN.XXXXXX)
BUILD_OUTPUT_DIR="/tmp/horus"
PROJECT_PATH="/Users/tefannastasa/workplace/horus"
REMOTE_DIR="/tmp/horus"
CONFIG="horus.toml"

case "$MODE" in
    dev | prod) ;;
    *)
        echo "usage: $0 [dev|prod]" >&2
        exit 2
        ;;
esac

r() {
    ssh -n -o ControlPath="$CTL" "$HOST" "$@"
}

cleanup() {
    # In prod the binary is meant to outlive this script, so only the dev run
    # gets killed on the way out.
    if [[ "$MODE" == "dev" ]]; then
        r pkill -x "$BIN" || true
    fi
    ssh -o ControlPath="$CTL" -O exit "$HOST" 2>/dev/null || true
}

cd "$PROJECT_PATH"
mkdir -p "$BUILD_OUTPUT_DIR"

# A prod deploy should be reproducible from a commit, not from whatever happens
# to be in the working tree.
if [[ "$MODE" == "prod" ]]; then
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "refusing to deploy: uncommitted changes" >&2
        exit 1
    fi
fi

echo "Validating implementation..."
cargo test

echo "Building binary for target: $TARGET"
if ! cargo zigbuild --release --target "$TARGET" >"$BUILD_OUTPUT_DIR/build.log" 2>&1; then
    cat "$BUILD_OUTPUT_DIR/build.log"
    exit 1
fi

TARGET_LOCATION="$PROJECT_PATH/target/$TARGET/release/$BIN"

ssh -o ControlMaster=yes -o ControlPath="$CTL" -o ControlPersist=no -fN "$HOST"
trap cleanup EXIT # clean the master socket after exit

case "$MODE" in
dev)
    r mkdir -p "$REMOTE_DIR"
    scp -o ControlPath="$CTL" "$TARGET_LOCATION" "$HOST:$REMOTE_DIR/$BIN" >/dev/null
    scp -o ControlPath="$CTL" "$CONFIG" "$HOST:$REMOTE_DIR/$CONFIG" >/dev/null
    echo "Artifact transferred to remote path: $REMOTE_DIR/$BIN"

    r chmod 744 "$REMOTE_DIR/$BIN"
    # Foreground, so logs stream back here and Ctrl-C stops it.
    ssh -o ControlPath="$CTL" "$HOST" \
        "RUST_LOG=debug $REMOTE_DIR/$BIN --config $REMOTE_DIR/$CONFIG"
    ;;

prod)
    REVISION=$(git rev-parse --short HEAD)

    r mkdir -p '~/bin'
    # Land beside the target and rename: a running binary can't be overwritten
    # in place (ETXTBSY), and mv within one filesystem is atomic.
    scp -o ControlPath="$CTL" "$TARGET_LOCATION" "$HOST:bin/$BIN.new" >/dev/null
    r "chmod 744 ~/bin/$BIN.new && mv ~/bin/$BIN.new ~/bin/$BIN"
    r "echo $REVISION > ~/bin/$BIN.version"

    # Config ships with the binary, so the repo stays the source of truth.
    scp -o ControlPath="$CTL" "$CONFIG" "$HOST:$CONFIG" >/dev/null

    r "systemctl --user restart $BIN"
    echo "Restarted $BIN at $REVISION, waiting for it to answer..."

    # restart succeeding only means the process started. Ask the HTTP layer.
    for _ in $(seq 1 10); do
        if r "curl -fsS -o /dev/null localhost:5067"; then
            echo "Deployed $REVISION"
            exit 0
        fi
        sleep 1
    done

    echo "Health check failed after restart:" >&2
    r "journalctl --user -u $BIN -n 30 --no-pager" >&2
    exit 1
    ;;
esac
