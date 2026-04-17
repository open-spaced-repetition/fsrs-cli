#!/usr/bin/env bash
# Integration test for all serve endpoints.
# Usage: ./tests/serve_test.sh
set -euo pipefail

PORT=17320
BASE="http://127.0.0.1:$PORT/api/v1"
PASS=0
FAIL=0
BINARY="${FSRS_BIN:-./target/release/fsrs}"

# Create a small test CSV
TEST_CSV=$(mktemp /tmp/fsrs_test_XXXXXX.csv)
cat > "$TEST_CSV" <<'EOF'
card_id,review_time,review_rating,review_state,review_duration
1001,1000000000000,3,0,5000
1001,1000086400000,3,1,4000
1001,1000518400000,3,2,3000
1001,1001382400000,2,2,8000
1002,1000000000000,3,0,6000
1002,1000172800000,3,1,3500
1002,1000604800000,4,2,2000
1003,1000000000000,1,0,10000
1003,1000086400000,3,1,5000
1003,1000259200000,3,2,4000
EOF

# Start server in background
"$BINARY" serve --port "$PORT" &
SERVER_PID=$!
trap "kill $SERVER_PID 2>/dev/null; wait $SERVER_PID 2>/dev/null; rm -f $TEST_CSV" EXIT

# Wait for server to be ready
for i in $(seq 1 30); do
    if curl -s "http://127.0.0.1:$PORT/docs" > /dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

assert_check() {
    local name="$1"
    local response="$2"
    local expected_field="$3"

    if echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); assert $expected_field" 2>/dev/null; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name"
        echo "    Response: $response"
        FAIL=$((FAIL + 1))
    fi
}

assert_json() {
    local name="$1"
    local method="$2"
    local url="$3"
    local body="${4:-}"
    local expected_field="$5"

    local response
    if [ "$method" = "GET" ]; then
        response=$(curl -s "$url")
    else
        response=$(curl -s -X POST "$url" -H "Content-Type: application/json" -d "$body")
    fi

    assert_check "$name" "$response" "$expected_field"
}

assert_csv_upload() {
    local name="$1"
    local url="$2"
    local csv_path="$3"
    local expected_field="$4"

    local response
    response=$(curl -s -X POST "$url" \
        -F "file=@$csv_path" \
        -F "timezone=UTC" \
        -F "day_cutoff=0")

    assert_check "$name" "$response" "$expected_field"
}

echo "Testing FSRS API endpoints on port $PORT"
echo

# ── schedule ──────────────────────────────────────────────
echo "[schedule]"
assert_json "next-states new card" POST "$BASE/schedule/next-states" \
    '{}' \
    "abs(d['again']['memory_state']['stability'] - 0.212) < 0.001"

assert_json "next-states existing card" POST "$BASE/schedule/next-states" \
    '{"memory_state":{"stability":2.3065,"difficulty":2.118104},"ivl":1}' \
    "abs(d['good']['memory_state']['stability'] - 7.3153) < 0.001"

# ── memory ────────────────────────────────────────────────
echo "[memory]"
assert_json "state" POST "$BASE/memory/state" \
    '{"reviews":[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5},{"rating":2,"delta_t":10}]}' \
    "abs(d['stability'] - 46.781) < 0.01"

assert_json "history" POST "$BASE/memory/history" \
    '{"reviews":[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5},{"rating":2,"delta_t":10}]}' \
    "len(d) == 4 and abs(d[0]['stability'] - 2.3065) < 0.001"

assert_json "retrievability" POST "$BASE/memory/retrievability" \
    '{"stability":10.0,"ivl":5.0}' \
    "abs(d - 0.9403) < 0.001"

assert_json "from-sm2" POST "$BASE/memory/from-sm2" \
    '{"ease_factor":2.5,"interval":30.0}' \
    "abs(d['stability'] - 30.0) < 0.001 and abs(d['difficulty'] - 6.093) < 0.01"

# ── params ────────────────────────────────────────────────
echo "[params]"
assert_json "default params" GET "$BASE/params/default" "" \
    "len(d['parameters']) == 21 and abs(d['parameters'][0] - 0.212) < 0.001"

# ── optimize ──────────────────────────────────────────────
echo "[optimize]"
ITEMS='{"items":[[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5}],[{"rating":3,"delta_t":0},{"rating":2,"delta_t":3}]]}'

assert_json "benchmark (JSON)" POST "$BASE/benchmark" \
    "$ITEMS" \
    "len(d) == 21"

assert_csv_upload "benchmark (CSV)" "$BASE/benchmark" "$TEST_CSV" \
    "len(d) == 21"

assert_csv_upload "optimize (CSV)" "$BASE/optimize" "$TEST_CSV" \
    "len(d) == 21"

# SSE: optimize with Accept: text/event-stream
echo "[optimize SSE]"
SSE_RESPONSE=$(curl -s -X POST "$BASE/optimize" \
    -F "file=@$TEST_CSV" \
    -F "timezone=UTC" \
    -F "day_cutoff=0" \
    -H "Accept: text/event-stream" \
    --max-time 60)

SSE_RESULT=$(echo "$SSE_RESPONSE" | grep -A1 "^event: result" | grep "^data:" | sed 's/^data: *//')
assert_check "optimize SSE stream" "$SSE_RESULT" "d['type'] == 'result' and len(d['data']) == 21"

# ── evaluate ──────────────────────────────────────────────
echo "[evaluate]"
assert_json "evaluate (JSON)" POST "$BASE/evaluate" \
    "$ITEMS" \
    "'log_loss' in d and 'rmse_bins' in d and d['log_loss'] > 0"

assert_csv_upload "evaluate (CSV)" "$BASE/evaluate" "$TEST_CSV" \
    "d['log_loss'] > 0 and d['rmse_bins'] > 0"

# ── simulate ──────────────────────────────────────────────
echo "[simulate]"
assert_json "simulate run" POST "$BASE/simulate/run" \
    '{"deck_size":100,"learn_span":30,"retention":0.9,"seed":42}' \
    "d['total_reviews'] == 373 and d['total_learned'] == 100 and d['days'] == 30"

assert_json "workload" POST "$BASE/simulate/workload" \
    '{"retention":0.9}' \
    "abs(d['expected_workload'] - 155.2765) < 0.1 and abs(d['retention'] - 0.9) < 0.001"

assert_json "optimal-retention" POST "$BASE/simulate/optimal-retention" \
    '{"deck_size":1000,"learn_span":30}' \
    "0.7 <= d['optimal_retention'] < 0.99"

echo
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
