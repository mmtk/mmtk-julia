set -xe

. $(dirname "$0")/common.sh

# Need a build_type argument
if [ $# -eq 0 ]
  then
    echo "No arguments supplied"
    exit 1
fi
# debug or release
build_type=$1
# Continue the build? We do not clean previous build in this case.
continue_build=$2

# helloworld.jl
HELLO_WORLD_JL=$BINDING_PATH/.github/scripts/hello_world.jl

# build MMTk
build_args=""
if [ "$build_type" == "release" ]; then
    build_args=$build_args" --release"
fi

cd $MMTK_JULIA_DIR
cargo build --features immix $build_args

cd $JULIA_PATH

# Clean first
if [[ -z "$continue_build" ]]; then
  make cleanall
fi

# Build
cp $BINDING_PATH/.github/scripts/Make.user $JULIA_PATH/
MMTK_BUILD=$build_type make
# Run hello world
$JULIA_PATH/julia $HELLO_WORLD_JL
