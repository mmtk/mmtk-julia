set -xe

. $(dirname "$0")/common.sh

# Need an index argument
if [ $# -eq 0 ]
  then
    echo "No arguments supplied"
    exit 1
fi

# We have an ordinal and a total number of runners. For example,
# we have ordinal=0, and a total of 2 runners. We only run tests with
# an even number (0, 2, 4, ...), and the other runner will run the
# odd numbered tests.
# To simply run all the tests, you can use ordinal=0 total=1
ordinal=$1
total=$2
echo "Run tests for #${ordinal} of ${total}"

# Patch some tests to skip
. $(dirname "$0")/ci-test-patching.sh

# Get all the tests
CHOOSE_TESTS_JL_PATH=$JULIA_PATH/test/choosetests.jl
CHOOSE_TESTS_JL_CONTENT=`cat $CHOOSE_TESTS_JL_PATH`

REGEX_PATTERN='.*const TESTNAMES = \[(.*)^\].*'

if [[ $CHOOSE_TESTS_JL_CONTENT =~ $REGEX_PATTERN ]]; then
    RAW_TEST_NAMES=${BASH_REMATCH[1]}
    # echo "matched: $RAW_TEST_NAMES"

    readarray -td, test_names <<< "$RAW_TEST_NAMES"
    declare test_names

    # the current index of test
    n=0

    for i in "${test_names[@]}"
    do
        # echo "Token: '$i'"
        test=`sed 's/\"\(.*\)\"/\1/' <<< $i`
        if [[ ! -z "$test" ]]; then
            echo $test

            # Should we skip some tests?
            # Ignore stdlib tests for now -- we run stdlib tests separately
            if [[ $test =~ "stdlib" ]]; then
                echo "-> Skip test"
                continue
            fi

            echo "-> (Test #$n for $ordinal/$total)"
            if [ $(( n % total )) -eq $ordinal ]; then
                echo "-> Run"
                ci_run_jl_test $test
            else
                echo "-> Skip"
            fi
            n=`expr $n + 1`
        fi
    done
else
    echo "Cannot find TESTNAMES in $CHOOSE_TESTS_JL_PATH"
    exit 1
fi
