#!/bin/sh
set -eu

# Windows invokes a non-interactive WSL shell, which may omit Cargo from PATH.
# Use an existing installation without changing the user's shell configuration.
if command -v cargo >/dev/null 2>&1; then
    tether_cargo=$(command -v cargo)
elif [ -n "${CARGO_HOME:-}" ] && [ -x "$CARGO_HOME/bin/cargo" ]; then
    tether_cargo="$CARGO_HOME/bin/cargo"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
    tether_cargo="$HOME/.cargo/bin/cargo"
else
    printf '%s\n' 'Cargo was not found in WSL PATH, CARGO_HOME/bin or ~/.cargo/bin. Use an existing Rust installation or pass -LinuxBinary to build.cmd.' >&2
    exit 127
fi

PATH="$(dirname "$tether_cargo"):$PATH"
export PATH
CARGO_TARGET_DIR=target/linux "$tether_cargo" build --locked --target x86_64-unknown-linux-musl "$@"
