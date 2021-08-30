#![allow(clippy::missing_safety_doc)]

use chrono::prelude::*;
use libc::{c_char, size_t};
use std::ptr;

/// A cron value managed by Rust.
///
/// Created with a UTF-8 string using `saffron_cron_parse`. Freed using `saffron_cron_free`.
pub struct Cron(saffron::Cron);

/// A future times iterator managed by Rust.
///
/// Created with an existing cron value using `saffron_cron_iter_from` or `saffron_cron_iter_after`.
/// Freed using `saffron_cron_iter_free`.
pub struct CronTimesIter(saffron::CronTimesIter);

fn box_it<T>(val: T) -> *mut T {
    Box::into_raw(val.into())
}

unsafe fn rebox_it<T>(ptr: *mut T) -> Box<T> {
    Box::from_raw(ptr)
}

/// Parses a UTF-8 string `s` with length `l` (without a null terminator) into a Cron value.
/// Returns null if:
///
/// * `s` is null,
///
/// * `s` is not valid UTF-8,
///
/// * `s` is not a valid cron expression,
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_parse(s: *const c_char, l: size_t) -> *const Cron {
    if s.is_null() {
        return ptr::null();
    }

    let slice = std::slice::from_raw_parts(s as *const u8, l);
    let string = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    match string.parse() {
        Ok(cron) => box_it(Cron(cron)) as _,
        Err(_) => ptr::null(),
    }
}

/// Frees a previously created cron value.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_free(c: *const Cron) {
    drop(rebox_it(c as *mut Cron))
}

/// Returns a bool indicating if the cron value contains any matching times.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_any(c: *const Cron) -> bool {
    (*c).0.any()
}

/// Returns a bool indicating if the cron value contains the given time in UTC non-leap seconds
/// since January 1st, 1970, 00:00:00.
///
/// The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_contains(c: *const Cron, s: i64) -> bool {
    let cron = &*c;
    if let Some(time) = Utc.timestamp_opt(s, 0).single() {
        cron.0.contains(time)
    } else {
        false
    }
}

/// Gets the next matching time in the cron value starting from the given time in UTC non-leap
/// seconds `s`. Returns a bool indicating if a next time exists, inserting the new timestamp into `s`.
///
/// The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_next_from(c: *const Cron, s: *mut i64) -> bool {
    let cron = &*c;
    if let Some(time) = Utc
        .timestamp_opt(*s, 0)
        .single()
        .and_then(|time| cron.0.next_from(time))
    {
        *s = time.timestamp();
        true
    } else {
        false
    }
}

/// Gets the next matching time in the cron value after the given time in UTC non-leap seconds `s`.
/// Returns a bool indicating if a next time exists, inserting the new timestamp into `s`.
///
/// The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_next_after(c: *const Cron, s: *mut i64) -> bool {
    let cron = &*c;
    if let Some(time) = Utc
        .timestamp_opt(*s, 0)
        .single()
        .and_then(|time| cron.0.next_after(time))
    {
        *s = time.timestamp();
        true
    } else {
        false
    }
}

/// Returns an iterator of future times starting from the specified timestamp `s` in UTC non-leap
/// seconds, or null if `s` is out of range of valid values.
///
/// The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_iter_from(c: *const Cron, s: i64) -> *mut CronTimesIter {
    let cron = &*c;
    if let Some(time) = Utc.timestamp_opt(s, 0).single() {
        box_it(CronTimesIter(cron.0.clone().iter_from(time)))
    } else {
        ptr::null_mut()
    }
}

/// Returns an iterator of future times starting after the specified timestamp `s` in UTC non-leap
/// seconds, or null if `s` is out of range of valid values.
///
/// The valid range for `s` is -8334632851200 <= `s` <= 8210298412799.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_iter_after(c: *const Cron, s: i64) -> *mut CronTimesIter {
    let cron = &*c;
    if let Some(time) = Utc.timestamp_opt(s, 0).single() {
        box_it(CronTimesIter(cron.0.clone().iter_after(time)))
    } else {
        ptr::null_mut()
    }
}

/// Gets the next timestamp in an cron times iterator, writing it to `s`. Returns a bool indicating
/// if a next time was written to `s`.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_iter_next(c: *mut CronTimesIter, s: *mut i64) -> bool {
    match (*c).0.next() {
        Some(time) => {
            *s = time.timestamp();
            true
        }
        None => false,
    }
}

/// Frees a previously created cron times iterator value.
#[no_mangle]
pub unsafe extern "C" fn saffron_cron_iter_free(c: *mut CronTimesIter) {
    drop(rebox_it(c))
}
