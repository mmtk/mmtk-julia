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
    # "Dates" -> skip
    # Test Failed at /home/runner/work/mmtk-julia/mmtk-julia/vm/julia/usr/share/julia/stdlib/v1.8/Dates/test/io.jl:45
    # Expression: repr(t) == shown
    # Evaluated: "Time(0, 0, 0, 1)" == "Dates.Time(0, 0, 0, 1)"
    # Seems to be an issue with their tests or runtime system: https://github.com/JuliaLang/julia/pull/29466
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
    # "Sockets" -> skip
    # getnameinfo(ip"0.1.1.1") == "0.1.1.1"
    # DNSError: ip"0.1.1.1", temporary failure (EAI_AGAIN)
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
