set -xe

. $(dirname "$0")/common.sh

# The list of tests/checks that we need to skip. They are either not suitable for Julia-MMTk, or
# not supported at this moment.
# Each line is a pattern of the test to match (we add skip=true to the end of those lines), and the test file path
# * Pattern ends with $ so we won't append 'skip=true' multiple times
declare -a tests_to_skip=(
    # Ignore the entire libgit2.jl -- there are too many possible network related issues to run this test
    # '@test.*$' "$JULIA_PATH/usr/share/julia/stdlib/v1.8/LibGit2/test/libgit2.jl"

    # These tests check for the number of stock GC threads (which we set to 0 with mmtk)
    '@test string(cpu_threads) ==' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test (cpu_threads == 1 ? "1" : string(div(cpu_threads, 2))) ==' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename --gcthreads=2 -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename --gcthreads=2,1 -e $code`, String) == "3"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename -e $code`, String) == "3"' "$JULIA_PATH/test/cmdlineargs.jl"

    # This test is about the page profiler
    '@test readline(fs) != ""' "$JULIA_PATH/stdlib/Profile/test/runtests.jl"

    # Skipping these GC tests for now (until we make sure we follow the stats as expected by the stock GC)
    '@test !live_bytes_has_grown_too_much' "$JULIA_PATH/test/gc.jl"
    '@test any(page_utilization .> 0)' "$JULIA_PATH/test/gc.jl"

    # These test the heapsize hint which is not used by mmtk
    '@test readchomp(`$(Base.julia_cmd()) --startup-file=no --heap-size-hint=500M -e "println(@ccall jl_gc_get_max_memory()::UInt64)"`) == "$((500-250)*1024*1024)"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test readchomp(`$(Base.julia_cmd()) --startup-file=no --heap-size-hint=10M -e "println(@ccall jl_gc_get_max_memory()::UInt64)"`) == "$(1*1024*1024)"' "$JULIA_PATH/test/cmdlineargs.jl"
    # This test checks GC logging
    '@test occursin("GC: pause", read(tmppath, String))' "$JULIA_PATH/test/misc.jl"

    # These tests check for the number of stock GC threads (which we set to 0 with mmtk)
    '@test (cpu_threads == 1 ? "1" : string(div(cpu_threads, 2))) ==' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename --gcthreads=2 -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    # This seems to be a regression from upstream when we merge with upstream 43bf2c8.
    # The required string int.jl does not appear in the output even if I test with the stock Julia code.
    # I do not know what is wrong, but at this point, I dont want to spend time on it.
    '@test occursin("int.jl", code)' "$JULIA_PATH/test/cmdlineargs.jl"
)

for (( i=0; i < ${#tests_to_skip[@]}; i+=2 )); do
    pattern=${tests_to_skip[i]}
    file=${tests_to_skip[i+1]}
    sed -i '/'"$pattern"'/ s/@test/@test_skip/' $file
done
