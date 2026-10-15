/* The Renyi embedding API for C hosts (decision AW46): a module loaded
 * with a grant answers calls to its public functions and nothing else.
 * The library is `renyi_c` (librenyi_c.so, librenyi_c.dylib, renyi_c.dll,
 * or the static librenyi_c.a). Texts cross as a pointer and a length,
 * UTF-8, not NUL-terminated; a text the library answers belongs to its
 * handle and is valid until the next call on that handle. A handle is
 * for one thread at a time. The guide is docs/embedding.md section 7.
 */
#ifndef RENYI_H
#define RENYI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The version of this API: additive changes keep it. */
#define RENYI_API_VERSION 1

/* How a call ended. */
#define RENYI_OK 0          /* the value as JSON is the text */
#define RENYI_FAILED 1      /* the function failed; the failure is the text */
#define RENYI_CRASHED 2     /* the message and the location are the text */
#define RENYI_EXITED 3      /* environment.exit; the code in renyi_exit_code */
#define RENYI_REFUSED 4     /* nothing ran; the reason is the text */
#define RENYI_OVER_MEMORY 5 /* the budget in renyi_memory_limit, the peak in renyi_memory_used */

/* A loaded module: opaque, one per module, freed by renyi_free_sandbox. */
typedef struct renyi_sandbox renyi_sandbox;

int renyi_api_version(void);
const char *renyi_version(void);
size_t renyi_version_len(void);

/* A module from its source file, or from a text whose imports resolve
 * from `name`, with a grant: the JSON object {"grant": "...",
 * "memory": "..."} as a text, or the empty text for a grant of nothing.
 * Null when the load fails, with the reason in renyi_last_error. */
renyi_sandbox *renyi_load(const char *path, size_t path_len,
                          const char *grant, size_t grant_len);
renyi_sandbox *renyi_load_source(const char *name, size_t name_len,
                                 const char *text, size_t text_len,
                                 const char *grant, size_t grant_len);
void renyi_free_sandbox(renyi_sandbox *sandbox);

/* Why the last load on this thread answered null. */
const char *renyi_last_error(void);
size_t renyi_last_error_len(void);

/* What the last call on the handle answered. */
const char *renyi_text(const renyi_sandbox *sandbox);
size_t renyi_text_len(const renyi_sandbox *sandbox);

/* A public function called with its arguments as one JSON array text
 * (each decoded against the parameter's declared type, in the codec of
 * std.json); the status, and the value as JSON or the reason in the
 * text. The guards are a JSON array with, per argument, the capability
 * of the grant whose `only to` guard the argument carries, or null;
 * the empty text for none. */
int renyi_call(renyi_sandbox *sandbox, const char *name, size_t name_len,
               const char *arguments, size_t arguments_len);
int renyi_call_guarded(renyi_sandbox *sandbox, const char *name, size_t name_len,
                       const char *arguments, size_t arguments_len,
                       const char *guards, size_t guards_len);
int renyi_exit_code(const renyi_sandbox *sandbox);
uint64_t renyi_memory_limit(const renyi_sandbox *sandbox);
uint64_t renyi_memory_used(const renyi_sandbox *sandbox);

/* The module described, each into the handle's text: its name, the
 * grant's capabilities as a `needs` clause spells them, the checker's
 * warnings, and the public functions as a JSON array of objects (name,
 * signature, purpose, parameters, needs, exposed_as_tool, input_schema,
 * callable). */
void renyi_module(renyi_sandbox *sandbox);
void renyi_grant(renyi_sandbox *sandbox);
void renyi_warnings(renyi_sandbox *sandbox);
void renyi_functions(renyi_sandbox *sandbox);

#ifdef __cplusplus
}
#endif

#endif
