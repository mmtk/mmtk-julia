set -xe

. $(dirname "$0")/common.sh

rustup toolchain install $RUSTUP_TOOLCHAIN
rustup target add i686-unknown-linux-gnu --toolchain $RUSTUP_TOOLCHAIN
rustup component add clippy --toolchain $RUSTUP_TOOLCHAIN
rustup component add rustfmt --toolchain $RUSTUP_TOOLCHAIN
rustup override set $RUSTUP_TOOLCHAIN
