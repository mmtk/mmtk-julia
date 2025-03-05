set -xe

. $(dirname "$0")/common.sh

# plan to use
plan=$1

export MMTK_PLAN=$plan

# Get all the tests
CHOOSE_TESTS_JL_PATH=$JULIA_PATH/test/choosetests.jl
CHOOSE_TESTS_JL_CONTENT=`cat $CHOOSE_TESTS_JL_PATH`

REGEX_PATTERN='.*const TESTNAMES = \[([^\[]*)^\].*'

if [[ $CHOOSE_TESTS_JL_CONTENT =~ $REGEX_PATTERN ]]; then
    RAW_TEST_NAMES=${BASH_REMATCH[1]}

    readarray -td, test_names <<< "$RAW_TEST_NAMES"
    declare test_names

    for i in "${test_names[@]}"
    do
        # echo "Token: '$i'"
        test=`sed 's/\"\(.*\)\"/\1/' <<< $i`
        if [[ ! -z "$test" ]]; then
            echo $test

            # Should we skip some tests?
            # Ignore stdlib tests for now -- we run stdlib tests separately
            if [[ $test =~ "stdlib" ]]; then
                echo "-> Skip stdlib"
                continue
            fi

            if [[ $test =~ "compiler_extras" ]]; then
                # Skipping compiler_extras for now
                echo "-> Skip compiler_extras"
                continue
            fi

            echo "-> Run"
            ci_run_jl_test $test
        fi
    done
else
    echo "Cannot find TESTNAMES in $CHOOSE_TESTS_JL_PATH"
    exit 1
fi
