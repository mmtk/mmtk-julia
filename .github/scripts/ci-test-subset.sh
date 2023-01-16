set -e

. $(dirname "$0")/common.sh

# Need an index argument
if [ $# -eq 0 ]
  then
    echo "No arguments supplied"
    exit 1
fi
ordinal=$1
total=$2
echo "Run tests for #${ordinal} of ${total}"

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
            echo "-> (Test #$n for $ordinal/$total)"
            if [ $(( n % total )) -eq $ordinal ]; then
                echo "-> Run"
                JULIA_NUM_THREADS=1 $JULIA_PATH/julia $JULIA_PATH/test/runtests.jl --exit-on-error $test
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
