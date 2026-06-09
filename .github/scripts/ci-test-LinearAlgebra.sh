# Running LinearAlgebra as a separate item
# Given it takes on average more than 2.5h to run 

set -e

. $(dirname "$0")/common.sh

# plan to use
plan=$1

export MMTK_PLAN=$plan

total_mem=$(free -m | awk '/^Mem:/ {print $2}')
mem_threshold=512 # use 0.5Gb as a threshold for the max rss based on the total free memory
total_mem_restricted=$((total_mem- mem_threshold))
num_workers=1
export JULIA_TEST_MAXRSS_MB=$((total_mem_restricted/ num_workers))

# Just use default herustics.
unset MMTK_MIN_HSIZE_G
unset MMTK_MAX_HSIZE_G

echo "-> Run single threaded"
ci_run_jl_test "LinearAlgebra" $num_workers
