#!/usr/bin/env bash
# Manual end-to-end test for semrouter POC-001
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/../.."
BINARY="$ROOT/target/release/semrouter"

if [ ! -f "$BINARY" ]; then
  echo "Building semrouter..."
  (cd "$ROOT" && cargo build --release)
fi

run_test() {
  local label="$1"
  local input="$2"
  local expected_route="$3"

  result=$(cd "$ROOT" && "$BINARY" route --compact "$input")
  selected=$(echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('selected_route','null'))")
  status=$(echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('status','?'))")
  top_score=$(echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['confidence']['top_score'])")

  if [ "$selected" = "$expected_route" ]; then
    echo "PASS [$label] → $selected (score=$top_score, status=$status)"
  else
    echo "FAIL [$label] expected=$expected_route got=$selected (score=$top_score, status=$status)"
    echo "  Full output: $result"
  fi
}

echo "=== semrouter POC-001 end-to-end tests ==="
echo ""

# Agent routing
run_test "coding"               "Help me debug this Python error"                       "coding"
run_test "coding_agent"         "Build me a full REST API with authentication in Rust"  "coding_agent"
run_test "research"             "Look up recent papers on transformer architectures"    "research"
run_test "research_agent"       "Research and compile a competitive landscape report"   "research_agent"

# Model routing
run_test "model_high_reasoning" "complex reasoning analytical decide strategy"          "model_routing_high_reasoning"
run_test "model_cheap"          "simple cheap fast generate quick"                      "model_routing_cheap"

# Second-brain
run_test "second_brain_capture" "Save this idea to my second brain"                    "second_brain_capture"
run_test "second_brain_retrieval" "Find my notes about distributed systems"            "second_brain_retrieval"

# Misc
run_test "strategy_planning"    "Help me plan a product launch strategy for Q3"        "strategy_planning"
run_test "summarization"        "Summarize this document in three bullet points"       "summarization"

echo ""
echo "=== Done ==="
