#!/usr/bin/env bash
set -euo pipefail

REPO="simonw/micropython-wasm"
VERSION="0.1a2"
FILENAME="micropython.wasm"

OUT_DIR="$(dirname "$0")/../wasm-runtimes"
mkdir -p "$OUT_DIR"

echo "Downloading MicroPython WASI binary..."
curl --fail --location \
  "https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}" \
  -o "${OUT_DIR}/${FILENAME}"

sha256sum "${OUT_DIR}/${FILENAME}"

echo "Inspecting exports..."
wasm-objdump -x "${OUT_DIR}/${FILENAME}" | grep -A 100 "Export"

echo "Done. Binary at: ${OUT_DIR}/${FILENAME}"
