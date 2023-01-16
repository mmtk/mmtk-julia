set -xe

. $(dirname "$0")/common.sh

# Test with 1 worker
. $(dirname "$0")/ci-test-subset.sh 0 1
