collect_descendant_pids() {
    local pid=$1
    local child

    while read -r child; do
        [ -n "$child" ] || continue
        echo "$child"
        collect_descendant_pids "$child"
    done < <(ps -o pid= --ppid "$pid")
}

dump_stack_traces() {
    local root_pid=$1
    local pid
    local pid_list
    local pids=("$root_pid")

    while read -r pid; do
        [ -n "$pid" ] || continue
        pids+=("$pid")
    done < <(collect_descendant_pids "$root_pid")

    echo "::warning::Build is still running after ${CI_STACK_DUMP_AFTER_SECS}s. Dumping stack traces."

    pid_list=$(IFS=,; echo "${pids[*]}")

    echo "::group::Process tree"
    ps -o pid=,ppid=,stat=,comm=,args= -p "$pid_list" || true
    echo "::endgroup::"

    if ! command -v gdb >/dev/null 2>&1; then
        echo "gdb is unavailable, attempting to install it for stack dumping"
        sudo apt-get update || true
        sudo apt-get install -y gdb || true
    fi

    if ! command -v gdb >/dev/null 2>&1; then
        echo "::warning::gdb is unavailable, skipping stack dump"
        return
    fi

    for pid in "${pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "::group::Stack trace for pid $pid"
            sudo gdb -q -batch \
                -ex "set pagination off" \
                -ex "thread apply all bt" \
                -p "$pid" || true
            echo "::endgroup::"
        fi
    done
}

start_stack_dump_watchdog() {
    local delay_secs=$1

    (
        sleep "$delay_secs"
        if kill -0 "$$" 2>/dev/null; then
            dump_stack_traces "$$"
        fi
    ) &
    STACK_DUMP_WATCHDOG_PID=$!
}

stop_stack_dump_watchdog() {
    if [ -n "${STACK_DUMP_WATCHDOG_PID:-}" ]; then
        kill "$STACK_DUMP_WATCHDOG_PID" 2>/dev/null || true
        wait "$STACK_DUMP_WATCHDOG_PID" 2>/dev/null || true
    fi
}

enable_stack_dump_watchdog() {
    if [ -n "${CI_STACK_DUMP_AFTER_SECS:-}" ] && [ "$CI_STACK_DUMP_AFTER_SECS" -gt 0 ]; then
        start_stack_dump_watchdog "$CI_STACK_DUMP_AFTER_SECS"
        trap stop_stack_dump_watchdog EXIT
    fi
}

enable_stack_dump_watchdog
