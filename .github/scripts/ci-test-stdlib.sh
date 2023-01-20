set -xe

. $(dirname "$0")/common.sh

cd $JULIA_PATH
make test-stdlib
