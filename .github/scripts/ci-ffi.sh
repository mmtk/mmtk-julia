set -ex

. $(dirname "$0")/common.sh

pushd $MMTK_JULIA_DIR

make regen-bindgen-ffi

popd

