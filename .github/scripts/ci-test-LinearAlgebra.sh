# Running LinearAlgebra as a separate item
# Given it takes on average more than 2.5h to run 

set -e

. $(dirname "$0")/common.sh

echo "-> Run single threaded"
ci_run_jl_test "LinearAlgebra" 1