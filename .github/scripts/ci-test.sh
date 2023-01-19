set -xe

. $(dirname "$0")/common.sh

# Test with 1 worker
# . $(dirname "$0")/ci-test-subset.sh 0 1

# Patch some tests to skip
. $(dirname "$0")/ci-test-patching.sh

cd $JULIA_PATH
export JULIA_TEST_MAXRSS_MB=3000
make testall3
