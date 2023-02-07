set -xe

. $(dirname "$0")/common.sh

export RUSTFLAGS="-D warnings"

pushd $BINDING_PATH/mmtk

# Currently we have many warnings from clippy, and we have PRs sitting there to be merged.
# I am concerned that enabling this will cause merge conflicts everywhere.
# However, we should enable this as soon as we can.
# cargo clippy
# cargo clippy --release

cargo fmt -- --check
popd
