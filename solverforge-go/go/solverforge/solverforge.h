#ifndef SOLVERFORGE_H
#define SOLVERFORGE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>

/**
 * GoBridge implements LanguageBridge for Go
 *
 * This bridge maintains a registry of Go object/function references and
 * provides methods to interact with them via callbacks into Go code.
 */
typedef struct GoBridge GoBridge;

/**
 * C-compatible error structure
 */
typedef struct CError {
  /**
   * Error message (owned C string)
   */
  char *message;
  /**
   * Error code
   */
  uint32_t code;
} CError;

/**
 * Create a new GoBridge instance
 *
 * Returns a pointer to a new GoBridge, or null on failure.
 *
 * # Safety
 *
 * The returned pointer must be freed with `solverforge_bridge_free`.
 */
struct GoBridge *solverforge_bridge_new(void);

/**
 * Free a GoBridge instance
 *
 * # Safety
 *
 * - `bridge` must be a valid pointer returned from `solverforge_bridge_new`
 * - `bridge` must not be used after this call
 * - This function must only be called once per bridge
 */
void solverforge_bridge_free(struct GoBridge *bridge);

/**
 * Register a Go object and get its handle ID
 *
 * # Parameters
 *
 * - `bridge`: The GoBridge instance
 * - `go_ref_id`: The Go-side object reference ID
 * - `out_handle`: Output parameter for the handle ID
 * - `out_error`: Output parameter for error (null on success)
 *
 * Returns `true` on success, `false` on failure.
 *
 * # Safety
 *
 * - `bridge` must be a valid GoBridge pointer
 * - `out_handle` must be a valid pointer to u64
 * - `out_error` must be a valid pointer to *mut CError
 */
bool solverforge_register_object(struct GoBridge *bridge,
                                 uint64_t go_ref_id,
                                 uint64_t *out_handle,
                                 struct CError **out_error);

/**
 * Free a CError instance
 *
 * # Safety
 *
 * - `error` must be a valid CError pointer
 * - `error` must not be used after this call
 * - This function must only be called once per error
 */
void solverforge_error_free(struct CError *error);

/**
 * Get the number of registered objects in the bridge
 *
 * This is primarily for testing and debugging.
 *
 * # Safety
 *
 * - `bridge` must be a valid GoBridge pointer
 */
uintptr_t solverforge_bridge_object_count(struct GoBridge *bridge);

/**
 * Get the number of registered functions in the bridge
 *
 * This is primarily for testing and debugging.
 *
 * # Safety
 *
 * - `bridge` must be a valid GoBridge pointer
 */
uintptr_t solverforge_bridge_function_count(struct GoBridge *bridge);

#endif /* SOLVERFORGE_H */
