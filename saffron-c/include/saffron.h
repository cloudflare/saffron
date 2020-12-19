#ifndef SAFFRON_H
#define SAFFRON_H

#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * A cron value managed by Rust.
 *
 * Created with a UTF-8 string using `saffron_cron_parse`. Freed using `saffron_cron_free`.
 */
typedef struct Cron Cron;

/**
 * A future times iterator managed by Rust.
 *
 * Created with an existing cron value using `saffron_cron_iter_from` or `saffron_cron_iter_after`.
 * Freed using `saffron_cron_iter_free`.
 */
typedef struct CronTimesIter CronTimesIter;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Parses a UTF-8 string `s` with length `l` (without a null terminator) into a Cron value.
 * Returns null if:
 *
 * * `s` is null,
 *
 * * `s` is not valid UTF-8,
 *
 * * `s` is not a valid cron expression,
 */
const struct Cron *saffron_cron_parse(const char *s, size_t l);

/**
 * Frees a previously created cron value.
 */
void saffron_cron_free(const struct Cron *c);

/**
 * Returns a bool indicating if the cron value contains any matching times.
 */
bool saffron_cron_any(const struct Cron *c);

/**
 * Returns a bool indicating if the cron value contains the given time in UTC non-leap seconds
 * since January 1st, 1970, 00:00:00.
 *
 * The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
 */
bool saffron_cron_contains(const struct Cron *c, int64_t s);

/**
 * Gets the next matching time in the cron value starting from the given time in UTC non-leap
 * seconds `s`. Returns a bool indicating if a next time exists, inserting the new timestamp into `s`.
 *
 * The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
 */
bool saffron_cron_next_from(const struct Cron *c,
                            int64_t *s);

/**
 * Gets the next matching time in the cron value after the given time in UTC non-leap seconds `s`.
 * Returns a bool indicating if a next time exists, inserting the new timestamp into `s`.
 *
 * The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
 */
bool saffron_cron_next_after(const struct Cron *c, int64_t *s);

/**
 * Returns an iterator of future times starting from the specified timestamp `s` in UTC non-leap
 * seconds, or null if `s` is out of range of valid values.
 *
 * The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
 */
struct CronTimesIter *saffron_cron_iter_from(const struct Cron *c, int64_t s);

/**
 * Returns an iterator of future times starting after the specified timestamp `s` in UTC non-leap
 * seconds, or null if `s` is out of range of valid values.
 *
 * The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
 */
struct CronTimesIter *saffron_cron_iter_after(const struct Cron *c, int64_t s);

/**
 * Gets the next timestamp in an cron times iterator, writing it to `s`. Returns a bool indicating
 * if a next time was written to `s`.
 */
bool saffron_cron_iter_next(struct CronTimesIter *c, int64_t *s);

/**
 * Frees a previously created cron times iterator value.
 */
void saffron_cron_iter_free(struct CronTimesIter *c);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* SAFFRON_H */
