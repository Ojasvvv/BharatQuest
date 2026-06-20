/*
 * Apatheia QuickJS WASM Wrapper
 *
 * This C file wraps the QuickJS interpreter to provide a minimal FFI surface
 * for the Rust/Wasmtime host. It is compiled alongside QuickJS sources into
 * a single wasm32-wasi module.
 *
 * Exported functions:
 *   alloc_buffer(size_t len) -> void*
 *     Allocates a buffer in WASM linear memory for the host to write JS source into.
 *
 *   eval_js(void* ptr, size_t len) -> int
 *     Evaluates the JS string at (ptr, len). Output goes to stdout (fd 1).
 *     Error/exception info goes to stderr (fd 2).
 *     Returns:
 *       0 = success
 *       1 = JS runtime error (exception thrown during execution)
 *       2 = JS parse/syntax error
 *
 * The host (Rust/Wasmtime) captures stdout/stderr via WASI pipe configuration.
 *
 * We also export get_output_ptr/get_output_len and get_error_ptr/get_error_len
 * for direct linear-memory reads of the last execution's output/error buffers,
 * as a complement to the WASI stdio capture.
 *
 * NOTE: We do NOT link quickjs-libc.c (it requires termios, signals, etc.
 * that aren't available in WASI). Instead we provide our own minimal console
 * implementation directly in this wrapper.
 */

#include "vendor/quickjs/quickjs.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* ---------- Output/error capture buffers ---------- */

/* Maximum output buffer size (256 KiB). Prevents unbounded memory growth. */
#define MAX_OUTPUT_SIZE (256 * 1024)

static char *g_output_buf = NULL;
static size_t g_output_len = 0;

static char *g_error_buf = NULL;
static size_t g_error_len = 0;

static void reset_buffers(void) {
    if (g_output_buf) { free(g_output_buf); g_output_buf = NULL; }
    g_output_len = 0;
    if (g_error_buf) { free(g_error_buf); g_error_buf = NULL; }
    g_error_len = 0;
}

static void append_to_output(const char *s, size_t len) {
    if (g_output_len + len > MAX_OUTPUT_SIZE)
        len = MAX_OUTPUT_SIZE - g_output_len;
    if (len == 0) return;
    char *new_buf = realloc(g_output_buf, g_output_len + len);
    if (!new_buf) return;
    memcpy(new_buf + g_output_len, s, len);
    g_output_buf = new_buf;
    g_output_len += len;
}

static void append_to_error(const char *s, size_t len) {
    if (g_error_len + len > MAX_OUTPUT_SIZE)
        len = MAX_OUTPUT_SIZE - g_error_len;
    if (len == 0) return;
    char *new_buf = realloc(g_error_buf, g_error_len + len);
    if (!new_buf) return;
    memcpy(new_buf + g_error_len, s, len);
    g_error_buf = new_buf;
    g_error_len += len;
}

/* ---------- Custom console.log implementation ---------- */

/*
 * We install a custom JS `console.log` that writes to our output buffer
 * AND to stdout (for WASI pipe capture). This way the host gets output
 * via both mechanisms.
 */
static JSValue js_console_log(JSContext *ctx, JSValueConst this_val,
                               int argc, JSValueConst *argv) {
    for (int i = 0; i < argc; i++) {
        if (i > 0) {
            append_to_output(" ", 1);
            fputc(' ', stdout);
        }
        size_t len;
        const char *str = JS_ToCStringLen(ctx, &len, argv[i]);
        if (str) {
            append_to_output(str, len);
            fwrite(str, 1, len, stdout);
            JS_FreeCString(ctx, str);
        }
    }
    append_to_output("\n", 1);
    fputc('\n', stdout);
    fflush(stdout);
    return JS_UNDEFINED;
}

/* ---------- Custom fetch implementation ---------- */

__attribute__((import_module("env"), import_name("host_fetch_start")))
extern int host_fetch_start(const char* url_ptr, size_t url_len, size_t* out_len);

__attribute__((import_module("env"), import_name("host_fetch_read")))
extern void host_fetch_read(char* out_body);

static JSValue js_fetch(JSContext *ctx, JSValueConst this_val,
                        int argc, JSValueConst *argv) {
    if (argc < 1) return JS_ThrowTypeError(ctx, "fetch requires 1 argument");
    
    size_t url_len;
    const char *url_str = JS_ToCStringLen(ctx, &url_len, argv[0]);
    if (!url_str) return JS_EXCEPTION;
    
    size_t out_len = 0;
    int result = host_fetch_start(url_str, url_len, &out_len);
    JS_FreeCString(ctx, url_str);
    
    if (result == 0) {
        if (out_len > 0) {
            char* buf = malloc(out_len);
            if (!buf) return JS_ThrowOutOfMemory(ctx);
            host_fetch_read(buf);
            JSValue ret = JS_NewStringLen(ctx, buf, out_len);
            free(buf);
            return ret;
        } else {
            return JS_NewString(ctx, "");
        }
    } else {
        if (out_len > 0) {
            char* buf = malloc(out_len);
            if (!buf) return JS_ThrowOutOfMemory(ctx);
            host_fetch_read(buf);
            JSValue err = JS_ThrowInternalError(ctx, "%.*s", (int)out_len, buf);
            free(buf);
            return err;
        } else {
            return JS_ThrowInternalError(ctx, "Fetch failed");
        }
    }
}

/* Install console object with log/warn/error methods */
static void install_console(JSContext *ctx) {
    JSValue global = JS_GetGlobalObject(ctx);
    JSValue console = JS_NewObject(ctx);

    JS_SetPropertyStr(ctx, console, "log",
        JS_NewCFunction(ctx, js_console_log, "log", 1));
    JS_SetPropertyStr(ctx, console, "warn",
        JS_NewCFunction(ctx, js_console_log, "warn", 1));
    JS_SetPropertyStr(ctx, console, "error",
        JS_NewCFunction(ctx, js_console_log, "error", 1));
    JS_SetPropertyStr(ctx, console, "info",
        JS_NewCFunction(ctx, js_console_log, "info", 1));

    JS_SetPropertyStr(ctx, global, "console", console);
    
    JS_SetPropertyStr(ctx, global, "fetch", 
        JS_NewCFunction(ctx, js_fetch, "fetch", 1));
        
    JS_FreeValue(ctx, global);
}

/* ---------- Exported FFI functions ---------- */

/*
 * alloc_buffer: Allocate `len` bytes in WASM linear memory.
 * The host writes the JS source string into this buffer via memory.write().
 *
 * __attribute__((export_name("alloc_buffer"))) ensures this symbol is
 * exported from the WASM module regardless of linker settings.
 */
__attribute__((export_name("alloc_buffer")))
void* alloc_buffer(size_t len) {
    return malloc(len);
}

/*
 * free_buffer: Free a buffer previously allocated by alloc_buffer.
 */
__attribute__((export_name("free_buffer")))
void free_buffer(void *ptr) {
    free(ptr);
}

/*
 * eval_js: Evaluate the JS source at (ptr, len).
 *
 * Returns:
 *   0 = success
 *   1 = JS runtime error (exception thrown during execution)
 *   2 = JS parse/syntax error
 *
 * On error, exception details (type, message, stack trace) are written
 * to both stderr and the internal error buffer.
 */
__attribute__((export_name("eval_js")))
int eval_js(void *ptr, size_t len) {
    int result = 0;

    /* Reset output/error buffers from any previous call */
    reset_buffers();

    /* Create a fresh QuickJS runtime + context for this evaluation */
    JSRuntime *rt = JS_NewRuntime();
    if (!rt) {
        const char *msg = "Failed to create QuickJS runtime\n";
        append_to_error(msg, strlen(msg));
        fprintf(stderr, "%s", msg);
        return 1;
    }

    /* Set memory limit to 16 MiB to prevent runaway allocations */
    JS_SetMemoryLimit(rt, 16 * 1024 * 1024);
    /* Disable QuickJS internal C stack checking. WASM uses a separate shadow stack
       for locals that does not map to a single contiguous memory region in the way
       QuickJS expects, causing spurious "stack overflow" syntax errors during parse. */
    JS_SetMaxStackSize(rt, 0);

    JSContext *ctx = JS_NewContext(rt);
    if (!ctx) {
        const char *msg = "Failed to create QuickJS context\n";
        append_to_error(msg, strlen(msg));
        fprintf(stderr, "%s", msg);
        JS_FreeRuntime(rt);
        return 1;
    }

    /* Install console.log/warn/error */
    install_console(ctx);

    /* DEBUG: dump what we received */
    fprintf(stderr, "DEBUG eval_js: ptr=%p len=%zu\n", ptr, len);
    if (ptr && len > 0) {
        size_t dump_len = len < 64 ? len : 64;
        fprintf(stderr, "DEBUG first %zu bytes: [", dump_len);
        for (size_t i = 0; i < dump_len; i++) {
            fprintf(stderr, "%c", ((const char *)ptr)[i]);
        }
        fprintf(stderr, "]\n");
        fprintf(stderr, "DEBUG hex: ");
        for (size_t i = 0; i < dump_len; i++) {
            fprintf(stderr, "%02x ", (unsigned char)((const char *)ptr)[i]);
        }
        fprintf(stderr, "\n");
    }
    fflush(stderr);

    /* Evaluate the JS source */
    JSValue val = JS_Eval(ctx, (const char *)ptr, len, "<input>",
                          JS_EVAL_TYPE_GLOBAL);

    if (JS_IsException(val)) {
        /* Get the exception object */
        JSValue exc = JS_GetException(ctx);

        /* Try to determine if this was a SyntaxError (parse error) */
        int is_syntax_error = 0;
        JSValue exc_name = JS_GetPropertyStr(ctx, exc, "name");
        if (!JS_IsUndefined(exc_name)) {
            const char *name_str = JS_ToCString(ctx, exc_name);
            if (name_str) {
                if (strcmp(name_str, "SyntaxError") == 0)
                    is_syntax_error = 1;
                // DO NOT FREE name_str YET, WE NEED IT FOR DEBUGGING
                JS_FreeCString(ctx, name_str);
            }
        }
        // DO NOT FREE exc_name YET

        result = is_syntax_error ? 2 : 1;

        /* Format the exception message */
        JSValue exc_msg = JS_GetPropertyStr(ctx, exc, "message");
        const char *name_str_for_dbg = JS_ToCString(ctx, exc_name);
        const char *msg_str_for_dbg = JS_ToCString(ctx, exc_msg);
        fprintf(stderr, "DEBUG EXCEPTION NAME: %s\n", name_str_for_dbg ? name_str_for_dbg : "null");
        fprintf(stderr, "DEBUG EXCEPTION MSG: %s\n", msg_str_for_dbg ? msg_str_for_dbg : "null");
        if (name_str_for_dbg) JS_FreeCString(ctx, name_str_for_dbg);
        if (msg_str_for_dbg) JS_FreeCString(ctx, msg_str_for_dbg);
        JS_FreeValue(ctx, exc_msg);

        const char *exc_str = JS_ToCString(ctx, exc);
        if (exc_str) {
            append_to_error(exc_str, strlen(exc_str));
            fprintf(stderr, "%s", exc_str);
            JS_FreeCString(ctx, exc_str);
        }

        /* Get and append the stack trace if available */
        JSValue stack = JS_GetPropertyStr(ctx, exc, "stack");
        if (!JS_IsUndefined(stack)) {
            const char *stack_str = JS_ToCString(ctx, stack);
            if (stack_str && strlen(stack_str) > 0) {
                append_to_error("\n", 1);
                append_to_error(stack_str, strlen(stack_str));
                fprintf(stderr, "\n%s", stack_str);
                JS_FreeCString(ctx, stack_str);
            }
        }
        JS_FreeValue(ctx, stack);
        append_to_error("\n", 1);
        fprintf(stderr, "\n");

        JS_FreeValue(ctx, exc);
    }

    JS_FreeValue(ctx, val);
    fflush(stdout);
    fflush(stderr);

    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);

    return result;
}

/*
 * get_output_ptr / get_output_len: Let the host read the output buffer
 * directly from WASM linear memory. This is a complement to WASI stdout.
 */
__attribute__((export_name("get_output_ptr")))
void* get_output_ptr(void) {
    return g_output_buf;
}

__attribute__((export_name("get_output_len")))
size_t get_output_len(void) {
    return g_output_len;
}

/*
 * get_error_ptr / get_error_len: Let the host read the error buffer
 * directly from WASM linear memory.
 */
__attribute__((export_name("get_error_ptr")))
void* get_error_ptr(void) {
    return g_error_buf;
}

__attribute__((export_name("get_error_len")))
size_t get_error_len(void) {
    return g_error_len;
}
