set -ex

. $(dirname "$0")/common.sh

JULIA_URL=`cargo read-manifest --manifest-path=$BINDING_PATH/mmtk/Cargo.toml | python -c 'import json,sys; print(json.load(sys.stdin)["metadata"]["julia"]["julia_repo"])'`
JULIA_VERSION=`cargo read-manifest --manifest-path=$BINDING_PATH/mmtk/Cargo.toml | python -c 'import json,sys; print(json.load(sys.stdin)["metadata"]["julia"]["julia_version"])'`

rm -rf $JULIA_PATH
git clone $JULIA_URL $JULIA_PATH
git -C $JULIA_PATH checkout $JULIA_VERSION
