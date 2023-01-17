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
    '@test.*$' "$JULIA_PATH/usr/share/julia/stdlib/v1.8/LibGit2/test/libgit2.jl"
)

for (( i=0; i < ${#tests_to_skip[@]}; i+=2 )); do
    pattern=${tests_to_skip[i]}
    file=${tests_to_skip[i+1]}
    sed -i '/'"$pattern"'/s/$/ skip=true/' $file
done
