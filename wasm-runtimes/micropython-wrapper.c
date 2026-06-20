#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "py/mpconfig.h"
#include "py/compile.h"
#include "py/runtime.h"
#include "py/repl.h"
#include "py/gc.h"
#include "py/mperrno.h"

// MicroPython requires a static GC heap.
static char heap[1024 * 1024];

static char* input_buf = NULL;
static size_t input_len = 0;

static char* output_buf = NULL;
static size_t output_buf_len = 0;
static size_t output_buf_cap = 0;

// Host export 1
__attribute__((export_name("alloc_buffer")))
uint8_t* alloc_buffer(size_t len) {
    if (input_buf) {
        free(input_buf);
    }
    input_buf = malloc(len + 1);
    if (input_buf) {
        input_len = len;
    }
    return (uint8_t*)input_buf;
}

// Host export 3
__attribute__((export_name("read_output")))
uint8_t* read_output() {
    return (uint8_t*)output_buf;
}

static void append_output(const char* str, size_t len) {
    if (output_buf_len + len + 1 > output_buf_cap) {
        output_buf_cap = output_buf_len + len + 1024;
        output_buf = realloc(output_buf, output_buf_cap);
    }
    memcpy(output_buf + output_buf_len, str, len);
    output_buf_len += len;
    output_buf[output_buf_len] = '\0';
}

static void my_print_strn(void *env, const char *str, size_t len) {
    (void)env;
    append_output(str, len);
}

// Host export 2
__attribute__((export_name("eval_code")))
int32_t eval_code(size_t len) {
    if (!input_buf) return 4; // Memory error

    input_buf[len] = '\0';

    if (output_buf) {
        free(output_buf);
        output_buf = NULL;
    }
    output_buf_cap = 1024;
    output_buf = malloc(output_buf_cap);
    output_buf[0] = '\0';
    output_buf_len = 0;

    gc_init(heap, heap + sizeof(heap));
    mp_init();

    mp_print_t print;
    print.data = NULL;
    print.print_strn = my_print_strn;

    nlr_buf_t nlr;
    int ret_code = 0;

    if (nlr_push(&nlr) == 0) {
        // Prepare to capture output
        append_output("s", 1); // Mark as stdout

        mp_lexer_t *lex = mp_lexer_new_from_str_len(MP_QSTR__lt_stdin_gt_, input_buf, len, 0);
        mp_parse_tree_t parse_tree = mp_parse(lex, MP_PARSE_FILE_INPUT);
        mp_obj_t module_fun = mp_compile(&parse_tree, lex->source_name, true);
        mp_call_function_0(module_fun);

        nlr_pop();
        ret_code = 0; // Success
    } else {
        // Exception
        output_buf_len = 0; // Reset output
        output_buf[0] = '\0';
        append_output("e", 1); // Mark as error

        mp_obj_print_exception(&print, (mp_obj_t)nlr.ret_val);
        ret_code = 1; // Execution Error
    }

    mp_deinit();
    return ret_code;
}

// Stubs for functions that might be required by ports/unix
void nlr_jump_fail(void *val) {
    while (1);
}

void mp_hal_set_interrupt_char(int c) {}

void mp_hal_stdout_tx_strn(const char *str, size_t len) {
    append_output(str, len);
}
