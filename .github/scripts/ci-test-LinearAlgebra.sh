# Running LinearAlgebra as a separate item
# Given it takes on average more than 2.5h to run 

set -e

. $(dirname "$0")/common.sh

export MMTK_MAX_HSIZE_G=8
total_mem=$(free -m | awk '/^Mem:/ {print $2}')
mem_threshold=512 # use 0.5Gb as a threshold for the max rss based on the total free memory
total_mem_restricted=$((total_mem- mem_threshold))
num_workers=2
export JULIA_TEST_MAXRSS_MB=$((total_mem_restricted/ num_workers))

echo "-> Run single threaded"
ci_run_jl_test "LinearAlgebra" 2