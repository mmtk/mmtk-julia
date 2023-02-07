set -xe

. $(dirname "$0")/common.sh

# Build deubg
. $(dirname "$0")/ci-build.sh debug
# Build release
. $(dirname "$0")/ci-build.sh release
# Use release build to run tests
. $(dirname "$0")/ci-test-other.sh
. $(dirname "$0")/ci-test-stdlib.sh
