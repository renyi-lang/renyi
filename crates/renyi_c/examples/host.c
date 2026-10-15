/* A C host of a Renyi module (decision AW46): loads calc.ry beside this
 * file with a grant, prints its public functions, calls two of them and
 * shows a refusal. Build against the library of `cargo build -p renyi_c`:
 *
 *   cc host.c -I../include -L../../../target/debug -lrenyi_c -o host
 *   ./host calc.ry
 *
 * (on Linux add -Wl,-rpath,../../../target/debug, on Windows put
 * renyi_c.dll on the PATH). The output is examples/host.expected.
 */
#include <stdio.h>
#include <string.h>

#include "renyi.h"

static void show(const char *label, const renyi_sandbox *module) {
    printf("%s%.*s\n", label, (int)renyi_text_len(module), renyi_text(module));
}

static void call(renyi_sandbox *module, const char *name, const char *arguments) {
    fflush(stdout); /* what the module prints to the console comes in order */
    int status = renyi_call(module, name, strlen(name), arguments, strlen(arguments));
    printf("%s%s -> status %d: ", name, arguments, status);
    show("", module);
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "calc.ry";
    const char *grant = "{\"grant\": \"console\", \"memory\": \"16 megabytes\"}";
    renyi_sandbox *module = renyi_load(path, strlen(path), grant, strlen(grant));
    if (module == NULL) {
        fprintf(stderr, "cannot load %s: %.*s\n", path, (int)renyi_last_error_len(),
                renyi_last_error());
        return 1;
    }
    /* the version on standard error: the expected output does not change
     * with a release */
    fprintf(stderr, "%.*s, API %d\n", (int)renyi_version_len(), renyi_version(),
            renyi_api_version());
    renyi_module(module);
    show("module: ", module);
    renyi_grant(module);
    show("grant: ", module);
    renyi_functions(module);
    show("functions: ", module);
    call(module, "add", "[2, 3]");
    call(module, "shout", "[\"hello\"]");
    call(module, "shift", "[{\"across\": 1, \"down\": 2}, 10]");
    call(module, "add", "[\"two\", 3]");
    call(module, "risky", "[-1]");
    renyi_free_sandbox(module);
    return 0;
}
