#!/usr/bin/env bash
# ── SparrowLib JNI Sample: build & run ──────────────────────────────
# Prerequisites: CMake build complete (libSparrowLib.so/dylib in target/debug/)
# Usage: ./run.sh [input.json] [output_dir]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

INPUT="${1:-data/input/albano.json}"
OUTDIR="${2:-output}"

echo "=== Building Java classes ==="
mkdir -p out
javac -encoding UTF-8 -d out java/com/sparrow/*.java java-sample/SampleRunner.java

echo ""
echo "=== Running Sample ==="

case "$(uname -s)" in
    Darwin)  export DYLD_LIBRARY_PATH="target/debug${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}";;
    Linux)   export LD_LIBRARY_PATH="target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}";;
esac

java -Djava.library.path=target/debug -cp out com.sparrow.sample.SampleRunner "$INPUT" "$OUTDIR"
