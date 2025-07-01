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
# max-moving vs non-moving
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

if [ "$moving_feature" == "max_moving" ]; then
    MOVING=1
    ALWAYS_MOVING=1
    MAX_MOVING=1
else
    MOVING=0
    ALWAYS_MOVING=0
    MAX_MOVING=0
fi

cd $JULIA_PATH
# Clean first
make cleanall
# This will build the binding in MMTK_JULIA_DIR (defined in common.sh), and link it
# when building Julia, instead of using the set version defined in Julia itself 
cp $BINDING_PATH/.github/scripts/Make.user $JULIA_PATH/
MMTK_MOVING=$MOVING MMTK_ALWAYS_MOVING=$ALWAYS_MOVING MMTK_MAX_MOVING=$MAX_MOVING MMTK_PLAN=$plan MMTK_BUILD=$build_type make
# Run hello world
$JULIA_PATH/julia $HELLO_WORLD_JL
