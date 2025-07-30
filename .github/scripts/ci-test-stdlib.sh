# We run stdlib tests separately, as it takes long and some tests are failling.
# Julia's make file also treats stdlib special. It is reasonable that we treat them differently.

set -e

. $(dirname "$0")/common.sh

# plan to use
plan=$1

export MMTK_PLAN=$plan

# These tests seem to fail. We skip them.
declare -a tests_to_skip=(
    # Test Failed at /home/runner/work/mmtk-julia/mmtk-julia/vm/julia/usr/share/julia/stdlib/v1.8/Dates/test/io.jl:45
    # Expression: repr(t) == shown
    # Evaluated: "Time(0, 0, 0, 1)" == "Dates.Time(0, 0, 0, 1)"
    # Seems to be an issue with their tests or runtime system: https://github.com/JuliaLang/julia/pull/29466
    "Dates"
    # getnameinfo(ip"0.1.1.1") == "0.1.1.1"
    # DNSError: ip"0.1.1.1", temporary failure (EAI_AGAIN)
    "Sockets"
    # LoadError: No active project
    # See https://github.com/JuliaLang/julia/issues/50055.
    # FIXME: We should run this test when the above issue is resolved.
    "Pkg",
    "SparseArrays"
    # Running LinearAlgebra in a separate job
    "LinearAlgebra"
    # Skipping Distributed tests
    "Distributed"

    # Skipping tests that fail for max moving Immix
    # see https://github.com/mmtk/mmtk-julia/issues/259
    "Artifacts"
    "Downloads"
    "REPL"
    "TOML"
    "Random"
    "LibCURL"
    "Mmap"
    "SharedArrays"
    "LazyArtifacts"
)
# These tests need multiple workers.
declare -a tests_with_multi_workers=(
    "Pkg"
)
# These tests run with a single worker
declare -a tests_with_single_worker=(
    "SparseArrays",
)

stdlib_path=$JULIA_PATH/usr/share/julia/stdlib

# They should have one directory in the path, like v1.8. The actual libraries are under that directory.
stdlib_version_path=$(find $stdlib_path -mindepth 1 -maxdepth 1)
# Should be exactly one directory
if [ $(find $stdlib_path -mindepth 1 -maxdepth 1 | wc -l) -ne 1 ]; then
  echo "Error: We expect to fine EXACTLY one directory under "$stdlib_path
  echo "We found"
  echo $stdlib_version_path
  exit 1
fi

# max-moving vs non-moving
is_moving=$2
moving_feature=${is_moving,,}

for dir in $(find $stdlib_version_path -depth -mindepth 1 -type d -o -type l)
do
    # if there is a runtests.jl, we run it.
    if [ -e "$dir/test/runtests.jl" ]; then
        # Get the basename such as Dates/Sockets/LinearAlgebra/etc
        test=$(echo "$dir" | xargs -I {} basename {})
        echo "Run stdlib tests: "$test

        # Skip some tests
        if [[ "${tests_to_skip[@]}" =~ "$test" ]]; then
            echo "-> Skip"
            continue
        fi

        if [[ "${tests_with_multi_workers[@]}" =~ "$test" ]]; then
            echo "-> Run multi threaded"
            ci_run_jl_test $test 2 $moving_feature
            continue
        fi

        if [[ "${tests_with_single_worker[@]}" =~ "$test" ]]; then
            echo "-> Run single threaded"
            ci_run_jl_test $test 1 $moving_feature
            continue
        fi

        ci_run_jl_test $test 1 $moving_feature
    fi
done
