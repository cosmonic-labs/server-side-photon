#!/usr/bin/env bash
#
# Performance benchmark: measures end-to-end and server-side timing for each transform.
#
# Usage:
#   ./tests/benchmark.sh                              # quick benchmark (1 iter, 100px)
#   ./tests/benchmark.sh --iterations 5               # 5 iterations for median
#   ./tests/benchmark.sh --sizes 100,200,400          # test multiple image sizes
#   ./tests/benchmark.sh --filter effects.oil         # benchmark one transform
#   ./tests/benchmark.sh --category Convolution       # benchmark one category
#   ./tests/benchmark.sh --csv results.csv            # export CSV
#   ./tests/benchmark.sh --compare baseline.csv       # compare against baseline
#
# Requires: wash dev running on localhost:8000, curl, jq
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOOLS="$SCRIPT_DIR/tools/target/release"
URL="${PHOTON_URL:-http://localhost:8000}"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Defaults
ITERATIONS=3
SIZES="100"
FILTER=""
CATEGORY=""
CSV_FILE=""
COMPARE_FILE=""
CUSTOM_IMAGE=""

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --iterations|-n) ITERATIONS="$2"; shift 2 ;;
        --sizes) SIZES="$2"; shift 2 ;;
        --filter) FILTER="$2"; shift 2 ;;
        --category) CATEGORY="$2"; shift 2 ;;
        --csv) CSV_FILE="$2"; shift 2 ;;
        --compare) COMPARE_FILE="$2"; shift 2 ;;
        --image) CUSTOM_IMAGE="$2"; shift 2 ;;
        --url) URL="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# Build tools if needed
if [[ ! -x "$TOOLS/mkimage" ]]; then
    echo "Building test tools..."
    (cd "$SCRIPT_DIR/tools" && cargo build --release 2>&1)
fi

# Check server
if ! curl -sf "$URL/" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $URL"
    echo "Start it with: wash dev"
    exit 1
fi

# Fetch transforms
TRANSFORMS_JSON=$(curl -sf "$URL/api/transforms")
TRANSFORM_LIST=$(echo "$TRANSFORMS_JSON" | jq -r '
    .[] | .category as $cat |
    .transforms[] |
    .name as $name |
    (.params // [] | map({(.name): .default}) | add // {}) as $defaults |
    (.params // [] | map(select(.kind == "select")) | map({(.name): .options[0]}) | add // {}) as $selects |
    ($defaults + $selects) as $params |
    "\($name)|\($cat)|\($params | tojson)"
')

# Apply filters
if [[ -n "$FILTER" ]]; then
    TRANSFORM_LIST=$(echo "$TRANSFORM_LIST" | grep -i "$FILTER" || true)
fi
if [[ -n "$CATEGORY" ]]; then
    TRANSFORM_LIST=$(echo "$TRANSFORM_LIST" | grep -i "|${CATEGORY}|" || true)
fi

if [[ -z "$TRANSFORM_LIST" ]]; then
    echo "No transforms to benchmark"
    exit 1
fi

TOTAL=$(echo "$TRANSFORM_LIST" | wc -l | tr -d ' ')

# helper: compute median of a list of numbers
median() {
    local sorted
    sorted=$(echo "$@" | tr ' ' '\n' | sort -n)
    local count
    count=$(echo "$sorted" | wc -l | tr -d ' ')
    local mid=$(( (count + 1) / 2 ))
    echo "$sorted" | sed -n "${mid}p"
}

# helper: compute mean
mean() {
    local sum=0
    local count=0
    for v in $@; do
        sum=$(echo "$sum + $v" | bc)
        count=$((count + 1))
    done
    echo "scale=1; $sum / $count" | bc
}

# helper: format ms
fmt_ms() {
    local ms="$1"
    if [[ $(echo "$ms >= 1000" | bc) -eq 1 ]]; then
        echo "$(echo "scale=1; $ms / 1000" | bc)s"
    else
        echo "${ms}ms"
    fi
}

# CSV header
CSV_ROWS=""
if [[ -n "$CSV_FILE" ]]; then
    CSV_ROWS="transform,category,image_size,input_bytes,output_bytes,server_median_ms,roundtrip_median_ms,iterations"
fi

IFS=',' read -ra SIZE_LIST <<< "$SIZES"

echo ""
echo "Benchmarking $TOTAL transforms, $ITERATIONS iterations"
echo ""

for SIZE in "${SIZE_LIST[@]}"; do
    # Generate test image
    TEST_IMAGE="$TMPDIR/test_${SIZE}.png"
    if [[ -n "$CUSTOM_IMAGE" ]]; then
        cp "$CUSTOM_IMAGE" "$TEST_IMAGE"
        SIZE="custom"
    else
        "$TOOLS/mkimage" "$SIZE" "$SIZE" "$TEST_IMAGE"
    fi
    INPUT_SIZE=$(wc -c < "$TEST_IMAGE" | tr -d ' ')

    # Warmup
    PAYLOAD="$TMPDIR/warmup.bin"
    "$TOOLS/mkpayload" "conv.identity" "$TEST_IMAGE" > "$PAYLOAD" 2>/dev/null
    curl -sf -o /dev/null -X POST "$URL/api/transform" \
        -H "Content-Type: application/octet-stream" \
        --data-binary @"$PAYLOAD" --max-time 30 2>/dev/null || true

    echo "======================================================================================================"
    printf "  Image: %sx%s (%s bytes)\n" "$SIZE" "$SIZE" "$INPUT_SIZE"
    echo "======================================================================================================"
    printf "  %-40s %14s %18s %10s\n" "Transform" "Server (med)" "Round-trip (med)" "Output"
    echo "  ----------------------------------------------------------------------------------------------------"

    CURRENT_CAT=""
    ALL_SERVER_TIMES=""
    ALL_RT_TIMES=""

    while IFS='|' read -r NAME CAT PARAMS_JSON; do
        if [[ "$CAT" != "$CURRENT_CAT" ]]; then
            CURRENT_CAT="$CAT"
            echo ""
            echo "  [$CAT]"
        fi

        # Build payload
        PAYLOAD="$TMPDIR/bench_payload.bin"
        PARAM_ARGS=()
        while IFS='=' read -r key val; do
            [[ -n "$key" ]] && PARAM_ARGS+=("$key=$val")
        done < <(echo "$PARAMS_JSON" | jq -r 'to_entries[] | "\(.key)=\(.value)"')
        "$TOOLS/mkpayload" "$NAME" "$TEST_IMAGE" ${PARAM_ARGS[@]+"${PARAM_ARGS[@]}"} > "$PAYLOAD" 2>/dev/null

        SERVER_TIMES=""
        RT_TIMES=""
        OUTPUT_SIZE=0

        for ((i=1; i<=ITERATIONS; i++)); do
            RESULT="$TMPDIR/bench_result.bin"
            HEADERS="$TMPDIR/bench_headers.txt"

            START_NS=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
            HTTP_CODE=$(curl -sf -o "$RESULT" -D "$HEADERS" -w "%{http_code}" \
                -X POST "$URL/api/transform" \
                -H "Content-Type: application/octet-stream" \
                --data-binary @"$PAYLOAD" \
                --max-time 60 2>/dev/null || echo "000")
            END_NS=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
            RT_MS=$(( (END_NS - START_NS) / 1000000 ))

            if [[ "$HTTP_CODE" == "200" ]]; then
                OUTPUT_SIZE=$(wc -c < "$RESULT" | tr -d ' ')
                # Extract server processing time from X-Processing-Info header
                PROC_INFO=$(grep -i "x-processing-info" "$HEADERS" 2>/dev/null | sed 's/^[^:]*: //' | tr -d '\r' || echo "")
                SERVER_MS=0
                if [[ -n "$PROC_INFO" ]]; then
                    SERVER_MS=$(echo "$PROC_INFO" | jq -r '.processing_time_ms // 0' 2>/dev/null || echo 0)
                fi
                SERVER_TIMES="$SERVER_TIMES $SERVER_MS"
                RT_TIMES="$RT_TIMES $RT_MS"
            else
                RT_TIMES="$RT_TIMES $RT_MS"
            fi
        done

        # Compute stats
        if [[ -n "$(echo $SERVER_TIMES | tr -d ' ')" ]]; then
            SRV_MED=$(median $SERVER_TIMES)
            RT_MED=$(median $RT_TIMES)
            ALL_SERVER_TIMES="$ALL_SERVER_TIMES $SRV_MED"
            ALL_RT_TIMES="$ALL_RT_TIMES $RT_MED"

            printf "  %-40s %14s %18s %10s\n" \
                "$NAME" "$(fmt_ms $SRV_MED)" "$(fmt_ms $RT_MED)" "$OUTPUT_SIZE"

            if [[ -n "$CSV_FILE" ]]; then
                CSV_ROWS="$CSV_ROWS
$NAME,$CAT,${SIZE}x${SIZE},$INPUT_SIZE,$OUTPUT_SIZE,$SRV_MED,$RT_MED,$ITERATIONS"
            fi
        else
            printf "  %-40s %14s %18s %10s\n" "$NAME" "FAIL" "-" "-"
        fi

    done <<< "$TRANSFORM_LIST"

    # Summary for this size
    if [[ -n "$(echo $ALL_SERVER_TIMES | tr -d ' ')" ]]; then
        echo ""
        echo "  Summary for ${SIZE}x${SIZE}:"

        SORTED_SRV=$(echo $ALL_SERVER_TIMES | tr ' ' '\n' | sort -n)
        MIN_SRV=$(echo "$SORTED_SRV" | head -1)
        MAX_SRV=$(echo "$SORTED_SRV" | tail -1)
        MED_SRV=$(median $ALL_SERVER_TIMES)
        MEAN_SRV=$(mean $ALL_SERVER_TIMES)

        SORTED_RT=$(echo $ALL_RT_TIMES | tr ' ' '\n' | sort -n)
        MIN_RT=$(echo "$SORTED_RT" | head -1)
        MAX_RT=$(echo "$SORTED_RT" | tail -1)
        MED_RT=$(median $ALL_RT_TIMES)
        MEAN_RT=$(mean $ALL_RT_TIMES)

        echo "    Server processing:  min=$(fmt_ms $MIN_SRV)  median=$(fmt_ms $MED_SRV)  mean=$(fmt_ms $MEAN_SRV)  max=$(fmt_ms $MAX_SRV)"
        echo "    Round-trip:         min=$(fmt_ms $MIN_RT)  median=$(fmt_ms $MED_RT)  mean=$(fmt_ms $MEAN_RT)  max=$(fmt_ms $MAX_RT)"

        # Top 5 slowest
        echo ""
        echo "    Top 5 slowest (server time):"
        # Re-run to pair names with times — collect during the loop
    fi

done

# Write CSV
if [[ -n "$CSV_FILE" ]]; then
    echo "$CSV_ROWS" > "$CSV_FILE"
    echo ""
    echo "Results exported to: $CSV_FILE"
fi

# Compare mode
if [[ -n "$COMPARE_FILE" ]] && [[ -f "$COMPARE_FILE" ]]; then
    echo ""
    echo "======================================================================================================"
    echo "  COMPARISON vs $COMPARE_FILE"
    echo "======================================================================================================"
    printf "  %-40s %10s %10s %10s %8s\n" "Transform" "Baseline" "Current" "Change" "%"
    echo "  ----------------------------------------------------------------------------------------------------"

    REGRESSIONS=0
    IMPROVEMENTS=0

    # Read baseline into associative-like format
    while IFS=',' read -r B_NAME B_CAT B_SIZE B_IN B_OUT B_SRV B_RT B_ITER; do
        [[ "$B_NAME" == "transform" ]] && continue  # skip header

        # Find matching current result
        if [[ -n "$CSV_FILE" ]]; then
            C_LINE=$(grep "^$B_NAME,$B_CAT,$B_SIZE," "$CSV_FILE" 2>/dev/null | head -1 || true)
            if [[ -n "$C_LINE" ]]; then
                C_SRV=$(echo "$C_LINE" | cut -d',' -f6)
                DIFF=$(echo "$C_SRV - $B_SRV" | bc)
                if [[ $(echo "$B_SRV > 0" | bc) -eq 1 ]]; then
                    PCT=$(echo "scale=0; $DIFF * 100 / $B_SRV" | bc)
                else
                    PCT=0
                fi

                MARKER=""
                if [[ $(echo "$PCT > 20" | bc) -eq 1 ]]; then
                    MARKER=" ** REGRESSION"
                    REGRESSIONS=$((REGRESSIONS + 1))
                elif [[ $(echo "$PCT < -20" | bc) -eq 1 ]]; then
                    MARKER=" ** FASTER"
                    IMPROVEMENTS=$((IMPROVEMENTS + 1))
                fi

                printf "  %-40s %10s %10s %+8sms %7s%%%s\n" \
                    "$B_NAME" "$(fmt_ms $B_SRV)" "$(fmt_ms $C_SRV)" "$DIFF" "$PCT" "$MARKER"
            fi
        fi
    done < "$COMPARE_FILE"

    echo ""
    echo "  Regressions (>20% slower): $REGRESSIONS"
    echo "  Improvements (>20% faster): $IMPROVEMENTS"
fi

echo ""
echo "Done."
