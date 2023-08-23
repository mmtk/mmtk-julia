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

    # This test checks GC logging
    '@test occursin("GC: pause", read(tmppath, String))' "$JULIA_PATH/test/misc.jl"

    # These tests check for the number of stock GC threads (which we set to 0 with mmtk)
    '@test (cpu_threads == 1 ? "1" : string(div(cpu_threads, 2))) ==
          read(`$exename --threads auto -e $code`, String) ==
          read(`$exename --threads=auto -e $code`, String) ==
          read(`$exename -tauto -e $code`, String) ==
          read(`$exename -t auto -e $code`, String)' "$JULIA_PATH/test/cmdlineargs.jl"

    '@test read(`$exename --gcthreads=2 -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"

    '@test read(`$exename -e $code`, String) == "2"' "$JULIA_PATH/test/cmdlineargs.jl"
)

for (( i=0; i < ${#tests_to_skip[@]}; i+=2 )); do
    pattern=${tests_to_skip[i]}
    file=${tests_to_skip[i+1]}
    sed -i '/'"$pattern"'/ s/@test/@test_skip/' $file
done
