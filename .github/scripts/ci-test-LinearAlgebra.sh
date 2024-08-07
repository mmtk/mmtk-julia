# Running LinearAlgebra as a separate item
# Given it takes on average more than 2.5h to run 

set -e

. $(dirname "$0")/common.sh

export MMTK_MAX_HSIZE_G=10
total_mem=$(free -m | awk '/^Mem:/ {print $2}')
mem_threshold=1536 # use 1.5Gb as a threshold for the max rss based on the total free memory
export JULIA_TEST_MAXRSS_MB=$((total_mem- mem_threshold))

echo "-> Run single threaded"
ci_run_jl_test "LinearAlgebra" 1