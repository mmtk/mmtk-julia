set -xe

cur=$(realpath $(dirname "$0"))

# Build debug - skip this. Otherwise it takes too long
# cd $cur
# ./ci-build.sh debug

# Build release
cd $cur
./ci-build.sh release

# Use release build to run tests
cd $cur
./ci-test-other.sh
cd $cur
./ci-test-stdlib.sh
