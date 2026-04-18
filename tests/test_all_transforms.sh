#!/usr/bin/env bash
#
# Test suite: runs every transform against the API and validates results.
#
# Usage:
#   ./tests/test_all_transforms.sh                  # test all transforms
#   ./tests/test_all_transforms.sh --save           # save output images to tests/output/
#   ./tests/test_all_transforms.sh --filter oil     # test only transforms matching "oil"
#   ./tests/test_all_transforms.sh --size 400       # use 400x400 test image
#   ./tests/test_all_transforms.sh --image photo.png  # use a custom image
#
# Requires: wash dev running on localhost:8000, curl, jq
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOLS="$SCRIPT_DIR/tools/target/release"
URL="${PHOTON_URL:-http://localhost:8000}"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Defaults
SAVE=false
FILTER=""
SIZE=200
CUSTOM_IMAGE=""

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --save) SAVE=true; shift ;;
        --filter) FILTER="$2"; shift 2 ;;
        --size) SIZE="$2"; shift 2 ;;
        --image) CUSTOM_IMAGE="$2"; shift 2 ;;
        --url) URL="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# Build tools if needed
if [[ ! -x "$TOOLS/mkimage" ]] || [[ ! -x "$TOOLS/mkpayload" ]] || [[ ! -x "$TOOLS/check-png" ]]; then
    echo "Building test tools..."
    (cd "$SCRIPT_DIR/tools" && cargo build --release 2>&1)
fi

# Generate or use test image
TEST_IMAGE="$TMPDIR/test.png"
if [[ -n "$CUSTOM_IMAGE" ]]; then
    cp "$CUSTOM_IMAGE" "$TEST_IMAGE"
    echo "Using custom image: $CUSTOM_IMAGE ($(wc -c < "$TEST_IMAGE" | tr -d ' ') bytes)"
else
    "$TOOLS/mkimage" "$SIZE" "$SIZE" "$TEST_IMAGE"
fi

INPUT_SIZE=$(wc -c < "$TEST_IMAGE" | tr -d ' ')
INPUT_HASH=$(shasum -a 256 "$TEST_IMAGE" | cut -c1-16)

# Create output dir if saving
OUTPUT_DIR="$SCRIPT_DIR/output"
if $SAVE; then
    mkdir -p "$OUTPUT_DIR"
    echo "Saving outputs to: $OUTPUT_DIR"
fi

# Check server is up
if ! curl -sf "$URL/" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $URL"
    echo "Start it with: wash dev"
    exit 1
fi

# Fetch transform catalog
TRANSFORMS_JSON=$(curl -sf "$URL/api/transforms")
if [[ -z "$TRANSFORMS_JSON" ]]; then
    echo "ERROR: Could not fetch /api/transforms"
    exit 1
fi

# Extract all transforms as "name|label|category|default_params_json"
TRANSFORM_LIST=$(echo "$TRANSFORMS_JSON" | jq -r '
    .[] | .category as $cat |
    .transforms[] |
    .name as $name |
    .label as $label |
    (.params // [] | map({(.name): .default}) | add // {}) as $defaults |
    # For select-type params, use the first option
    (.params // [] | map(select(.kind == "select")) | map({(.name): .options[0]}) | add // {}) as $selects |
    ($defaults + $selects) as $params |
    "\($name)|\($label)|\($cat)|\($params | tojson)"
')

# Apply filter
if [[ -n "$FILTER" ]]; then
    TRANSFORM_LIST=$(echo "$TRANSFORM_LIST" | grep -i "$FILTER" || true)
    if [[ -z "$TRANSFORM_LIST" ]]; then
        echo "No transforms matching '$FILTER'"
        exit 1
    fi
fi

TOTAL=$(echo "$TRANSFORM_LIST" | wc -l | tr -d ' ')

echo ""
echo "Test image: ${SIZE}x${SIZE}, $INPUT_SIZE bytes (sha256: $INPUT_HASH)"
echo "Server: $URL"
echo "Transforms to test: $TOTAL"
echo ""

# Header
printf "%-45s %6s %5s %7s %10s %8s\n" "Transform" "Status" "PNG?" "Changed" "Size" "Time"
echo "---------------------------------------------------------------------------------------------------"

PASSED=0
FAILED=0
FAIL_LIST=""
CURRENT_CAT=""

while IFS='|' read -r NAME LABEL CATEGORY PARAMS_JSON; do
    # Category header
    if [[ "$CATEGORY" != "$CURRENT_CAT" ]]; then
        CURRENT_CAT="$CATEGORY"
        echo ""
        echo "  [$CATEGORY]"
    fi

    # Build payload
    PAYLOAD="$TMPDIR/payload.bin"
    RESULT="$TMPDIR/result.png"

    # Build params args for mkpayload
    PARAM_ARGS=()
    while IFS='=' read -r key val; do
        [[ -n "$key" ]] && PARAM_ARGS+=("$key=$val")
    done < <(echo "$PARAMS_JSON" | jq -r 'to_entries[] | "\(.key)=\(.value)"')

    "$TOOLS/mkpayload" "$NAME" "$TEST_IMAGE" ${PARAM_ARGS[@]+"${PARAM_ARGS[@]}"} > "$PAYLOAD" 2>/dev/null

    # Call the API and measure time
    HTTP_CODE=""
    TIME_MS=""
    START_NS=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
    HTTP_CODE=$(curl -sf -o "$RESULT" -w "%{http_code}" \
        -X POST "$URL/api/transform" \
        -H "Content-Type: application/octet-stream" \
        --data-binary @"$PAYLOAD" \
        --max-time 60 2>/dev/null || echo "000")
    END_NS=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
    TIME_MS=$(( (END_NS - START_NS) / 1000000 ))

    # Validate
    IS_OK=false
    IS_PNG=false
    IS_CHANGED=false
    RESULT_SIZE=0

    if [[ "$HTTP_CODE" == "200" ]]; then
        IS_OK=true
        RESULT_SIZE=$(wc -c < "$RESULT" | tr -d ' ')

        # Check PNG validity
        if "$TOOLS/check-png" "$RESULT" > /dev/null 2>&1; then
            IS_PNG=true
        fi

        # Check if output differs from input
        RESULT_HASH=$(shasum -a 256 "$RESULT" | cut -c1-16)
        if [[ "$RESULT_HASH" != "$INPUT_HASH" ]]; then
            IS_CHANGED=true
        fi
    fi

    # Determine pass/fail
    STATUS="PASS"
    FAIL_REASON=""
    if ! $IS_OK; then
        STATUS="FAIL"
        FAIL_REASON="HTTP $HTTP_CODE"
    elif ! $IS_PNG; then
        STATUS="FAIL"
        FAIL_REASON="invalid PNG"
    elif [[ "$RESULT_SIZE" -eq 0 ]]; then
        STATUS="FAIL"
        FAIL_REASON="empty"
    fi

    if [[ "$STATUS" == "PASS" ]]; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
        FAIL_LIST="$FAIL_LIST\n  $NAME: $FAIL_REASON"
    fi

    # Format output
    CHANGED_STR="yes"
    if ! $IS_CHANGED; then
        if [[ "$NAME" == "conv.identity" ]]; then
            CHANGED_STR="n/a"
        else
            CHANGED_STR="NO"
        fi
    fi

    SIZE_STR="$RESULT_SIZE"
    if ! $IS_OK; then SIZE_STR="ERR"; fi

    printf "  %-43s %6s %5s %7s %10s %6sms" \
        "$NAME" "$STATUS" \
        "$(if $IS_PNG; then echo yes; else echo NO; fi)" \
        "$CHANGED_STR" "$SIZE_STR" "$TIME_MS"

    if [[ -n "$FAIL_REASON" ]]; then
        printf "  (%s)" "$FAIL_REASON"
    fi
    echo ""

    # Save output
    if $SAVE && $IS_PNG; then
        SAFE_NAME=$(echo "$NAME" | tr '.' '_')
        cp "$RESULT" "$OUTPUT_DIR/${SAFE_NAME}.png"
    fi

done <<< "$TRANSFORM_LIST"

# Also test named filter presets
echo ""
echo "Testing named filter presets..."
PRESET_PASS=0
PRESET_FAIL=0
for PRESET in oceanic islands marine seagreen flagblue liquid diamante radio twenties rosetint mauve bluechrome vintage perfume serenity; do
    PAYLOAD="$TMPDIR/payload.bin"
    RESULT="$TMPDIR/result_preset.png"
    "$TOOLS/mkpayload" "filters.filter" "$TEST_IMAGE" "filter_name=$PRESET" > "$PAYLOAD" 2>/dev/null

    HTTP_CODE=$(curl -sf -o "$RESULT" -w "%{http_code}" \
        -X POST "$URL/api/transform" \
        -H "Content-Type: application/octet-stream" \
        --data-binary @"$PAYLOAD" \
        --max-time 30 2>/dev/null || echo "000")

    RESULT_SIZE=$(wc -c < "$RESULT" 2>/dev/null | tr -d ' ' || echo 0)

    if [[ "$HTTP_CODE" == "200" ]] && "$TOOLS/check-png" "$RESULT" > /dev/null 2>&1; then
        PRESET_PASS=$((PRESET_PASS + 1))
        printf "  filters.filter(%-12s)  PASS  %10s bytes\n" "$PRESET" "$RESULT_SIZE"
    else
        PRESET_FAIL=$((PRESET_FAIL + 1))
        printf "  filters.filter(%-12s)  FAIL  HTTP %s\n" "$PRESET" "$HTTP_CODE"
        FAILED=$((FAILED + 1))
        FAIL_LIST="$FAIL_LIST\n  filters.filter($PRESET): HTTP $HTTP_CODE"
    fi
done

PASSED=$((PASSED + PRESET_PASS))
TOTAL=$((TOTAL + 15))

# Summary
echo ""
echo "==================================================================================================="
echo "Results: $PASSED/$TOTAL passed, $FAILED failed"

if [[ $FAILED -gt 0 ]]; then
    echo ""
    echo "Failed transforms:"
    echo -e "$FAIL_LIST"
fi

if $SAVE; then
    echo ""
    echo "Output images saved to: $OUTPUT_DIR"
fi

exit $FAILED
