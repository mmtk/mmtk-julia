set -xe

. $(dirname "$0")/common.sh

. $(dirname "$0")/ci-test-subset.sh 0 1
. $(dirname "$0")/ci-test-stdlib.sh
