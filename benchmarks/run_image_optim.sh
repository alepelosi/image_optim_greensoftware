#!/usr/bin/env bash
set -euo pipefail

INPUT_DIR="${1:-test_images}"
RUN_DIR="runs/bench"

rm -rf "$RUN_DIR"
mkdir -p runs
cp -r "$INPUT_DIR" "$RUN_DIR"

START_SIZE_KB=$(du -sk "$RUN_DIR" | awk '{print $1}')

/usr/bin/time -p bundle exec bin/image_optim -r "$RUN_DIR" --no-svgo

END_SIZE_KB=$(du -sk "$RUN_DIR" | awk '{print $1}')

echo "start_size_kb=$START_SIZE_KB"
echo "end_size_kb=$END_SIZE_KB"
echo "saved_kb=$((START_SIZE_KB - END_SIZE_KB))"
