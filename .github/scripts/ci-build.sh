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
# plan to use
plan=$2
# moving vs non-moving
is_moving=$3

# helloworld.jl
HELLO_WORLD_JL=$BINDING_PATH/.github/scripts/hello_world.jl

# build MMTk
build_args=""
if [ "$build_type" == "release" ]; then
    build_args=$build_args" --release"
fi

plan_feature=${plan,,}
moving_feature=${is_moving,,}
if [ "$is_moving" == "Default" ]; then
    tpin_roots=1
else
    tpin_roots=0
fi

cd $MMTK_JULIA_DIR/mmtk
cargo build --features $plan_feature,$moving_feature $build_args

cd $JULIA_PATH

# Clean first
make cleanall
# Build
cp $BINDING_PATH/.github/scripts/Make.user $JULIA_PATH/
MMTK_PLAN=$plan MMTK_BUILD=$build_type MMTK_TPIN_ROOTS=$tpin_roots make
# Run hello world
$JULIA_PATH/julia $HELLO_WORLD_JL
