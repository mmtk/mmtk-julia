set -xe

. $(dirname "$0")/common.sh

export RUSTFLAGS="-D warnings"

pushd $BINDING_PATH/mmtk

cargo clippy
cargo clippy --release

cargo fmt -- --check
popd
