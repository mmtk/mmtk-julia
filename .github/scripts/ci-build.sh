set -xe

. $(dirname "$0")/common.sh

# helloworld.jl
HELLO_WORLD_JL=$BINDING_PATH/.github/scripts/hello_world.jl

# Do a debug build

# build MMTk
cd $MMTK_JULIA_DIR
cargo build --features immix
# build Julia
cd $JULIA_PATH
cp $BINDING_PATH/.github/scripts/Make.user $JULIA_PATH/
MMTK_BUILD=debug make
# Run hello world
$JULIA_PATH/julia $HELLO_WORLD_JL

# Do a release build again
make cleanall

# build MMTk
cd $MMTK_JULIA_DIR
cargo build --features immix --release
# build Julia
cd $JULIA_PATH
cp $BINDING_PATH/.github/scripts/Make.user $JULIA_PATH/
MMTK_BUILD=release make
# Run hello world
$JULIA_PATH/julia $HELLO_WORLD_JL
