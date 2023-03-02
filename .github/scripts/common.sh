BINDING_PATH=$(realpath $(dirname "$0"))/../..
JULIA_PATH=$BINDING_PATH/vm/julia

RUSTUP_TOOLCHAIN=`cat $BINDING_PATH/mmtk/rust-toolchain`
JULIA_TEST_ARGS='--check-bounds=yes --startup-file=no --depwarn=error'

# Julia binding requires these
export MMTK_JULIA_DIR=$BINDING_PATH

# Make sure we have enough heap to build Julia
export MMTK_MIN_HSIZE_G=0.5
export MMTK_MAX_HSIZE_G=4

ci_run_jl_test() {
    test=$1
    threads=$2

    # if no argument is given, use 2 as default
    if [ -z "$threads" ]; then
        threads=2
    fi

    cd $JULIA_PATH
    export JULIA_CPU_THREADS=$threads

    # Directly run runtests.jl: There could be some issues with some test suites. We better just use their build script.
    # $JULIA_PATH/julia $JULIA_TEST_ARGS $JULIA_PATH/test/runtests.jl --exit-on-error $test

    # Run with their build script
    make test-$test
}
