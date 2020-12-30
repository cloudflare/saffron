import * as _ from "./saffron_bg.wasm"; // unused because the wasm/js story sucks: https://github.com/rustwasm/wasm-bindgen/pull/2110
import { WasmCron, WasmCronTimesIter } from "./saffron_bg.js";

/**
 * An iterator over all matching dates for a cron value starting at or after a specific date.
 */
export class CronTimesIter {
  /** @private */
  static __wrap(iter) {
    const obj = Object.create(CronTimesIter.prototype);
    obj.iter = iter;

    return obj;
  }

  /**
   * Frees the underlying wasm memory associated with this object.
   */
  free() {
    const iter = this.iter;
    this.iter = null;

    iter.free();
  }

  /**
   * Gets the next matching time in the cron times iterator.
   * @returns {{ value: Date | undefined, done: boolean }}
   */
  next() {
    const next = this.iter.next();
    return {
      value: next,
      done: next == null,
    };
  }

  /**
   * Returns this instance.
   * @returns {CronTimesIter}
   */
  [Symbol.iterator]() {
    return this;
  }
}

/**
 * A parsed cron value. This can be used to check if a time matches the cron value or get an iterator
 * of all future times.
 */
export default class Cron {
  /**
   * Parses a cron expression into a cron value.
   *
   * @param {string} s The string value to parse
   * @throws If the string is not a valid cron expression
   */
  constructor(s) {
    this.value = new WasmCron(s);
  }

  /**
   * Parses a cron expression into a cron value and string description.
   * 
   * @param {string} s The string value to parse
   * @returns {[Cron, string]} A cron value and a string description
   * @throws If the string is not a valid cron expression
   */
  static parseAndDescribe(s) {
    let [cron, description] = WasmCron.parseAndDescribe(s);

    const obj = Object.create(Cron.prototype);
    obj.value = cron;

    return [obj, description];
  }

  /**
   * Frees the underlying wasm memory associated with this object.
   */
  free() {
    const value = this.value;
    this.value = null;

    value.free();
  }

  /**
   * Returns whether this cron value will match on any one time.
   *
   * If a cron value is used that only matches on a day of the month that's not contained in any
   * month specified, that cron value will technically be valid, but will never match a given time.
   *
   * @returns {boolean} `true` if the cron value contains at least one matching time, `false` otherwise
   *
   * @example
   * // returns true
   * new Cron("* * 29 2 *").any()
   *
   * // returns false, November doesn't have a 31st day
   * new Cron("* * 31 11 *").any()
   */
  any() {
    return this.value.any();
  }

  /**
   * Returns whether this cron value matches on the specified date.
   * @param {Date} date The date to check
   * @returns {boolean} `true` if the cron value matches on this date, `false` otherwise
   */
  contains(date) {
    return this.value.contains(date);
  }

  /**
   * Returns the next matching date starting from the given date. This includes the date given,
   * which will have seconds truncated if the minute matches the cron value.
   *
   * @param {Date} date The start date
   * @returns {Date | undefined} The next matching date starting from the start date, or `undefined`
   * if no date was found.
   */
  nextFrom(date) {
    return this.value.nextFrom(date);
  }

  /**
   * Returns the next matching date starting after the given date.
   *
   * @param {Date} date The start date
   * @returns {Date | undefined} The next matching date after the start date, or `undefined` if no
   * date was found.
   */
  nextAfter(date) {
    return this.value.nextAfter(date);
  }

  /**
   * Returns an iterator of all times starting at the specified date.
   * @param {Date} date The date to start the iterator from
   * @returns {CronTimesIter} An iterator of all times starting at the specified date
   */
  iterFrom(date) {
    const iter = WasmCronTimesIter.startFrom(this.value, date);
    return CronTimesIter.__wrap(iter);
  }

  /**
   * Returns an iterator of all times starting after the specified date.
   * @param {Date} date The date to start the iterator after
   * @returns {CronTimesIter} An iterator of all times starting after the specified date
   */
  iterAfter(date) {
    const iter = WasmCronTimesIter.startAfter(this.value, date);
    return CronTimesIter.__wrap(iter);
  }
}
