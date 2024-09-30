# Running LinearAlgebra as a separate item
# Given it takes on average more than 2.5h to run 

set -e

. $(dirname "$0")/common.sh

export MMTK_MAX_HSIZE_G=6
echo "-> Run single threaded"
ci_run_jl_test "LinearAlgebra" 1