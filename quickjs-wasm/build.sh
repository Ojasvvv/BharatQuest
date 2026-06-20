#!/usr/bin/env bash
# =============================================================================
# Apatheia QuickJS WASM Build Script
#
# Cross-compiles QuickJS + our wrapper to wasm32-wasi using wasi-sdk's clang.
#
# Pinned source: bellard/quickjs commit 04be246 (2026-06-04)
#
# Usage:
#   ./build.sh [path-to-wasi-sdk]
#
# If no argument is given, looks for wasi-sdk at:
#   1. $WASI_SDK_PATH environment variable
#   2. ../wasi-sdk-*  (sibling directory in the workspace)
#   3. /opt/wasi-sdk
#
# Output: build/quickjs.wasm
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ---------------------------------------------------------------------------
# Locate wasi-sdk
# ---------------------------------------------------------------------------
if [ -n "${1:-}" ]; then
    WASI_SDK="$1"
elif [ -n "${WASI_SDK_PATH:-}" ]; then
    WASI_SDK="$WASI_SDK_PATH"
else
    # Auto-detect in common locations
    WASI_SDK=""
    for candidate in \
        "$SCRIPT_DIR/../wasi-sdk-"* \
        /opt/wasi-sdk \
        /usr/local/wasi-sdk; do
        if [ -d "$candidate/bin" ] 2>/dev/null; then
            WASI_SDK="$(cd "$candidate" && pwd)"
            break
        fi
    done
fi

if [ -z "${WASI_SDK:-}" ] || [ ! -f "$WASI_SDK/bin/clang" ]; then
    echo "ERROR: wasi-sdk not found. Install it:"
    echo "  wget https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-25/wasi-sdk-25.0-x86_64-linux.tar.gz"
    echo "  tar xzf wasi-sdk-25.0-x86_64-linux.tar.gz"
    echo "  export WASI_SDK_PATH=\$(pwd)/wasi-sdk-25.0-x86_64-linux"
    exit 1
fi

CC="$WASI_SDK/bin/clang"
SYSROOT="$WASI_SDK/share/wasi-sysroot"
echo "Using wasi-sdk at: $WASI_SDK"
echo "Clang: $CC"
echo "Sysroot: $SYSROOT"

# ---------------------------------------------------------------------------
# Source files
# ---------------------------------------------------------------------------
# QuickJS core sources — minimal set for the interpreter.
# We EXCLUDE:
#   - qjs.c        (standalone REPL binary — we provide our own entry via wrapper.c)
#   - qjsc.c       (bytecode compiler tool — not needed at runtime)
#   - quickjs-libc.c  (requires termios.h, signal.h which aren't available in WASI;
#                       we provide our own console.log in wrapper.c instead)
#   - run-test262.c   (test harness)
#   - unicode_gen.c   (build-time code generation tool)
QJS_DIR="vendor/quickjs"
QJS_SOURCES=(
    "$QJS_DIR/quickjs.c"       # Core interpreter (parser, bytecode compiler, VM)
    "$QJS_DIR/libregexp.c"     # Regular expression engine
    "$QJS_DIR/libunicode.c"    # Unicode support tables and functions
    "$QJS_DIR/cutils.c"        # Utility functions (string, memory helpers)
    "$QJS_DIR/dtoa.c"          # Double-to-ASCII conversion (number formatting)
)

# Our wrapper that provides the FFI surface (alloc_buffer, eval_js, etc.)
WRAPPER="wrapper.c"

# ---------------------------------------------------------------------------
# Create stub headers for WASI-incompatible system headers
# ---------------------------------------------------------------------------
# dtoa.c includes <setjmp.h> but doesn't actually call setjmp/longjmp.
# WASI's setjmp.h errors out because WASM exception handling isn't standard.
# We provide a no-op stub that satisfies the #include without enabling sjlj.
mkdir -p build/stubs
cat > build/stubs/setjmp.h << 'EOF'
/* Stub setjmp.h for wasm32-wasi build.
 * dtoa.c includes this header but never calls setjmp/longjmp.
 * We provide empty typedefs to satisfy the preprocessor. */
#ifndef _STUB_SETJMP_H
#define _STUB_SETJMP_H
typedef int jmp_buf[1];
typedef int sigjmp_buf[1];
#define setjmp(env) 0
#define longjmp(env, val) ((void)0)
#define sigsetjmp(env, savesigs) 0
#define siglongjmp(env, val) ((void)0)
#endif
EOF

# ---------------------------------------------------------------------------
# Compiler flags — each one documented
# ---------------------------------------------------------------------------
CFLAGS=(
    # Target the WASM32 architecture with WASI system interface
    "--target=wasm32-wasi"

    # Point to the WASI sysroot for libc headers and libraries
    "--sysroot=$SYSROOT"

    # Put our stub headers FIRST in the include path so they override
    # the WASI sysroot's error-producing setjmp.h
    "-isystem" "build/stubs"

    # Optimize for size — reduces .wasm binary, important for fast InstancePre
    # compilation. -Os is chosen over -O2 because WASM module size directly
    # affects startup compilation time.
    "-Os"

    # Include path so wrapper.c can find QuickJS headers via "vendor/quickjs/..."
    "-I."

    # Version string to match our pinned commit
    "-DCONFIG_VERSION=\"2026-06-04\""

    # Suppress warnings that are noisy but harmless in cross-compilation
    "-Wno-implicit-function-declaration"
    "-Wno-int-conversion"

    # Single-threaded model — WASI doesn't support pthreads in preview1.
    # This prevents the compiler from emitting thread-local storage or
    # atomics that would require a threading runtime.
    "-mthread-model" "single"

    # Disable trapping math — WASM traps on some FP operations (like
    # converting NaN to int) that C expects to produce undefined behavior.
    # This flag makes the compiler emit non-trapping alternatives.
    "-fno-trapping-math"

    # Build as a WASI "reactor" module instead of a "command".
    # Commands export _start (which calls main and then proc_exit).
    # Reactors export _initialize (which runs CRT constructors only,
    # initializing malloc/stdio/etc.) and then the host calls our
    # exported FFI functions directly. This is essential because:
    #   1. We have no main() — we're a library
    #   2. The host needs libc initialized before calling alloc_buffer/eval_js
    #   3. _start would call proc_exit, trapping the WASM instance
    "-mexec-model=reactor"
)

# ---------------------------------------------------------------------------
# Linker flags
# ---------------------------------------------------------------------------
LDFLAGS=(
    # Export _initialize so the Wasmtime host can call it after instantiation.
    # _initialize runs __wasm_call_ctors (CRT global constructors) which
    # initializes the WASI libc (malloc, stdio, etc.). This is generated by
    # -mexec-model=reactor above.
    "-Wl,--export=_initialize"

    # Export our wrapper functions so they're visible to the Wasmtime host.
    # We use explicit exports rather than --export-all to minimize the
    # export table and avoid exposing internal QuickJS symbols.
    "-Wl,--export=alloc_buffer"
    "-Wl,--export=free_buffer"
    "-Wl,--export=eval_js"
    "-Wl,--export=get_output_ptr"
    "-Wl,--export=get_output_len"
    "-Wl,--export=get_error_ptr"
    "-Wl,--export=get_error_len"

    # Export memory so the host can read/write WASM linear memory.
    # wasm-ld uses --export-memory (not --export=<symbol>) for the
    # module's linear memory, since 'memory' is not a regular symbol.
    "-Wl,--export-memory"

    # Allow memory to grow up to 256 MiB (4096 pages × 64KiB).
    # Initial memory is set by the linker based on static data.
    "-Wl,--max-memory=268435456"

    # Strip debug info from the WASM binary to reduce size.
    # Wasmtime doesn't use DWARF debug info from WASM modules.
    "-Wl,--strip-all"
)

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
echo ""
echo "=== Compiling QuickJS + wrapper to wasm32-wasi ==="
echo "Sources: ${QJS_SOURCES[*]} $WRAPPER"
echo ""

# Compile all sources and link into a single WASM module in one step.
# This is simpler than separate compile+link for our use case.
"$CC" \
    "${CFLAGS[@]}" \
    "${LDFLAGS[@]}" \
    "${QJS_SOURCES[@]}" \
    "$WRAPPER" \
    -o build/quickjs.wasm

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
WASM_SIZE=$(stat -c%s build/quickjs.wasm 2>/dev/null || stat -f%z build/quickjs.wasm)
echo ""
echo "=== Build successful ==="
echo "Output: build/quickjs.wasm ($WASM_SIZE bytes, $(( WASM_SIZE / 1024 )) KiB)"
echo ""
echo "Exported functions:"
echo "  alloc_buffer(size_t len) -> void*"
echo "  free_buffer(void* ptr)"
echo "  eval_js(void* ptr, size_t len) -> int"
echo "  get_output_ptr() -> void*"
echo "  get_output_len() -> size_t"
echo "  get_error_ptr() -> void*"
echo "  get_error_len() -> size_t"
