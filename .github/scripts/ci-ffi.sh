set -ex

. $(dirname "$0")/common.sh

pushd $MMTK_JULIA_DIR

make regen-bindgen-ffi

if ! git diff --exit-code $MMTK_JULIA_DIR/mmtk/src/julia_types.rs; then
  echo "Rust FFI bindings in \`julia_types.rs\` are outdated. Run \`make regen-bindgen-ffi\` from the mmtk-julia directory and make sure to include the updated file in the pull request."
  exit 1
fi

popd

