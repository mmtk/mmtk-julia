set -ex

. $(dirname "$0")/common.sh

# We may later allow setting up a specific version of Julia using comments
# in the PR, but for now we just use the latest master from JuliaLang
JULIA_URL=https://github.com/JuliaLang/julia.git
JULIA_VERSION=master

rm -rf $JULIA_PATH
git clone $JULIA_URL $JULIA_PATH
git -C $JULIA_PATH checkout $JULIA_VERSION
