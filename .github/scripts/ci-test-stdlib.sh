# We run stdlib tests separately, as it takes long and some tests are failling.
# Julia's make file also treats stdlib special. It is reasonable that we treat them differently.

set -e

. $(dirname "$0")/common.sh

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
)
# These tests need multiple workers.
declare -a tests_with_multi_workers=(
    "Pkg"
)
# These tests run with a single worker
declare -a tests_with_single_worker=(
    "SparseArrays",
    "LinearAlgebra"
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
            ci_run_jl_test $test 2
            continue
        fi

        if [[ "${tests_with_single_worker[@]}" =~ "$test" ]]; then
            echo "-> Run single threaded"
            ci_run_jl_test $test 1
            continue
        fi

        ci_run_jl_test $test
    fi
done
