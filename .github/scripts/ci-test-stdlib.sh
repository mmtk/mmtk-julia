set -xe

. $(dirname "$0")/common.sh

# These tests seem to fail:
# LibGit2
# Dates/io
# FileWatching

declare -a stdlib_tests=(
    "ArgTools"
    "Artifacts"
    "Base64"
    "CRC32c"
    "CompilerSupportLibraries_jll"
    "Dates"
    "DelimitedFiles"
    "Distributed"
    "Downloads"
    "FileWatching"
    "Future"
    "GMP_jll"
    "InteractiveUtils"
    "LLVMLibUnwind_jll"
    "LazyArtifacts"
    "LibCURL"
    "LibCURL_jll"
    "LibGit2"
    "LibGit2_jll"
    "LibSSH2_jll"
    "LibUV_jll"
    "LibUnwind_jll"
    "Libdl"
    "LinearAlgebra"
    "Logging"
    "MPFR_jll"
    "Markdown"
    "MbedTLS_jll"
    "Mmap"
    "MozillaCACerts_jll"
    "NetworkOptions"
    "OpenBLAS_jll"
    "OpenLibm_jll"
    "PCRE2_jll"
    "Pkg"
    "Printf"
    "Profile"
    "REPL"
    "Random"
    "SHA"
    "Serialization"
    "SharedArrays"
    "Sockets"
    "SparseArrays"
    "Statistics"
    "SuiteSparse"
    "SuiteSparse_jll"
    "TOML"
    "Tar"
    "Test"
    "UUIDs"
    "Unicode"
    "Zlib_jll"
    "dSFMT_jll"
    "libLLVM_jll"
    "libblastrampoline_jll"
    "nghttp2_jll"
    "p7zip_jll"
)

cd $JULIA_PATH

for i in "${stdlib_tests[@]}"
do
    test="$i"
    echo "Run stdlib tests: "$test
    ci_run_jl_test $test
done

make test-stdlib
