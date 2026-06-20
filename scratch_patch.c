#include "vendor/quickjs/quickjs.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

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
