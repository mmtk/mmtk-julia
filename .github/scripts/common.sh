BINDING_PATH=$(realpath $(dirname "$0"))/../..
JULIA_PATH=$BINDING_PATH/vm/julia

RUSTUP_TOOLCHAIN=`cat $BINDING_PATH/mmtk/rust-toolchain`

# Julia binding requires these
export MMTK_JULIA_DIR=$BINDING_PATH/mmtk

# Make sure we have enough heap to build Julia
export MMTK_MIN_HSIZE_G=0.5
export MMTK_MAX_HSIZE_G=4
