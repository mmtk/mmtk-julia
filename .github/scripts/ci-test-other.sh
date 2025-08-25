set -xe

. $(dirname "$0")/common.sh

# plan to use
plan=$1

export MMTK_PLAN=$plan

# Get all the tests
CHOOSE_TESTS_JL_PATH=$JULIA_PATH/test/choosetests.jl
CHOOSE_TESTS_JL_CONTENT=`cat $CHOOSE_TESTS_JL_PATH`

REGEX_PATTERN='.*const TESTNAMES = \[([^\[]*)^\].*'

# max-moving vs non-moving
is_moving=$2
moving_feature=${is_moving,,}

declare -a max_moving_tests_to_skip=(
    # see https://github.com/mmtk/mmtk-julia/issues/259
    "abstractarray"
    "cmdlineargs"
    "Downloads"
    "read"
    "LibCURL"
    "loading"
    "misc"
)

if [[ $CHOOSE_TESTS_JL_CONTENT =~ $REGEX_PATTERN ]]; then
    RAW_TEST_NAMES=${BASH_REMATCH[1]}

    readarray -td, test_names <<< "$RAW_TEST_NAMES"
    declare test_names

    for i in "${test_names[@]}"
    do
        # echo "Token: '$i'"
        test=$(sed 's/\"\(.*\)\"/\1/' <<< "$i" | xargs)
        if [[ ! -z "$test" ]]; then
            echo $test

            # Should we skip some tests?
            # Ignore stdlib tests for now -- we run stdlib tests separately
            if [[ $test =~ "stdlib" ]]; then
                echo "-> Skip stdlib"
                continue
            fi

            if [[ "${max_moving_tests_to_skip[@]}" =~ "$test" ]]; then
                if [ "$moving_feature" == "max_moving" ]; then
                    echo "-> Skip"
                    continue
                fi
            fi

            if [[ $test =~ "compiler_extras" ]]; then
                # Skipping compiler_extras for now
                echo "-> Skip compiler_extras"
                continue
            fi

            if [[ $test =~ "rounding" ]]; then
                # Run rounding test with single thread and Julia's 
                # heap resizing (it OOMs with a fixed heap)
                echo "-> Run"
                ci_run_jl_test $test 1 $moving_feature
                continue
            fi

            if [[ $test =~ "ranges" ]]; then
                # Run ranges test with single thread and Julia's 
                # heap resizing (it OOMs with a fixed heap)
                echo "-> Run"
                ci_run_jl_test $test 1 $moving_feature
                continue
            fi

            echo "-> Run"
            ci_run_jl_test $test 2 $moving_feature
        fi
    done
else
    echo "Cannot find TESTNAMES in $CHOOSE_TESTS_JL_PATH"
    exit 1
fi
