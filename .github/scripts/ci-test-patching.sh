set -xe

. $(dirname "$0")/common.sh

# The list of tests/checks that we need to skip. They are either not suitable for Julia-MMTk, or
# not supported at this moment.
# Each line is a pattern of the test to match (we add skip=true to the end of those lines), and the test file path
# * Pattern ends with $ so we won't append 'skip=true' multiple times
declare -a tests_to_skip=(
    # These tests expect jl_gc_pool_alloc in the generated code which is wrong
    '@test occursin("jl_gc_pool_alloc", get_llvm(MutableStruct, Tuple{}))$' "$JULIA_PATH/test/compiler/codegen.jl"
    '@test occursin("jl_gc_pool_alloc", breakpoint_any_ir)$' "$JULIA_PATH/test/compiler/codegen.jl"

    # Ignore the entire libgit2.jl -- there are too many possible network related issues to run this test
    # '@test.*$' "$JULIA_PATH/usr/share/julia/stdlib/v1.8/LibGit2/test/libgit2.jl"

    # These tests check for the number of stock GC threads (which we set to 0 with mmtk)
    '@test (cpu_threads == 1 ? "1" : string(div(cpu_threads, 2))) ==' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename --gcthreads=2 -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename --gcthreads=2,1 -e $code`, String) == "3"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
    '@test read(`$exename -e $code`, String) == "3"' "$JULIA_PATH/test/cmdlineargs.jl"

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

    # These are failing for v1.9.2 on the stock Julia as well.
    '@test process_running(p)' "$JULIA_PATH/stdlib/Profile/test/runtests.jl"
    '@test occursin("Overhead ╎", s)' "$JULIA_PATH/stdlib/Profile/test/runtests.jl"
    '@test length(prof.allocs) >= 1' "$JULIA_PATH/stdlib/Profile/test/allocs.jl"
    '@test length(\[a for a in prof.allocs if a.type == MyType\]) >= 1' "$JULIA_PATH/stdlib/Profile/test/allocs.jl"
)

for (( i=0; i < ${#tests_to_skip[@]}; i+=2 )); do
    pattern=${tests_to_skip[i]}
    file=${tests_to_skip[i+1]}
    sed -i '/'"$pattern"'/ s/@test/@test_skip/' $file
done
