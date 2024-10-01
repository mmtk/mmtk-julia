set -xe

# Parse arguments: i and n
i=$1
n=$2

if [[ -z "$i" || -z "$n" ]]; then
    echo "Usage: $0 <i> <n>"
    exit 1
fi

. $(dirname "$0")/common.sh

# Get all the tests
CHOOSE_TESTS_JL_PATH=$JULIA_PATH/test/choosetests.jl
CHOOSE_TESTS_JL_CONTENT=`cat $CHOOSE_TESTS_JL_PATH`

REGEX_PATTERN='.*const TESTNAMES = \[([^\[]*)^\].*'

if [[ $CHOOSE_TESTS_JL_CONTENT =~ $REGEX_PATTERN ]]; then
    RAW_TEST_NAMES=${BASH_REMATCH[1]}

    readarray -td, test_names <<< "$RAW_TEST_NAMES"
    declare test_names

    # Calculate the total number of tests
    total_tests=${#test_names[@]}
    if [[ "$total_tests" -eq 0 ]]; then
        echo "No tests found."
        exit 1
    fi

    # Calculate start and end indices for the ith part
    part_size=$(( (total_tests + n - 1) / n ))  # This rounds up to ensure all tests are covered
    start_index=$(( i * part_size ))
    end_index=$(( (i + 1) * part_size ))

    if [[ "$start_index" -ge "$total_tests" ]]; then
        echo "No tests to run for this part."
        exit 0
    fi

    if [[ "$end_index" -gt "$total_tests" ]]; then
        end_index=$total_tests
    fi

    # Run the ith part of the tests
    for (( j=$start_index; j<$end_index; j++ ))
    do
        test=`sed 's/\"\(.*\)\"/\1/' <<< "${test_names[j]}"`
        if [[ ! -z "$test" ]]; then
            echo $j
            echo $test

            # Should we skip some tests?
            # Ignore stdlib tests for now -- we run stdlib tests separately
            if [[ $test =~ "stdlib" ]]; then
                echo "-> Skip stdlib"
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
