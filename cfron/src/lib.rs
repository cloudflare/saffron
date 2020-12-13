//! A "Quartz scheduler"-like cron parser powering Cron Triggers on Cloudflare Workers.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod parse;

use chrono::prelude::*;

use core::cmp;
use core::fmt::Debug;
use core::str::FromStr;

use self::parse::{CronExpr, ExprValue, OrsExpr};

/// Returns the number of days in the month, 28-31
fn days_in_month(date: Date<Utc>) -> u32 {
    match date.month() {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let year = date.year();
            if year % 4 != 0 {
                28
            } else if year % 100 != 0 {
                29
            } else if year % 400 != 0 {
                28
            } else {
                29
            }
        }
        _ => unreachable!(),
    }
}

trait TimePattern {
    /// A parsed time expression value
    type Expr;

    /// Compiles the expression into its most compressed form.
    fn compile(expr: Self::Expr) -> Self;

    /// Checks if the pattern contains the given DateTime.
    fn contains(&self, date: DateTime<Utc>) -> bool;
}

const DBG_BAD_PATTERN: &str = "Value mapped out of range of valid bit patterns";

macro_rules! debug_assert_pattern {
    ($pat:expr, $mask:expr) => {
        debug_assert!(($pat & !($mask)) == 0, DBG_BAD_PATTERN)
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum DaysOfWeekKind {
    /// An expression over a set of values, ranges, or steps
    Pattern,
    /// A '*' expression
    Star,
    /// A 'L' expression for the last day. One day is paired with this making it easier to access
    Last,
    /// A '#' expression for an nth day of the month. One day and one nth value is paired making it
    /// easier to access
    Nth,
}

/// A bit-mask of all the days of the week set in a cron expression.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct DaysOfWeek(DaysOfWeekKind, u8);
impl TimePattern for DaysOfWeek {
    type Expr = parse::DayOfWeekExpr;

    #[inline]
    fn compile(expr: Self::Expr) -> Self {
        match expr {
            parse::DayOfWeekExpr::All => Self(DaysOfWeekKind::Star, 0),
            parse::DayOfWeekExpr::Last(day) => Self(DaysOfWeekKind::Last, u8::from(day)),
            parse::DayOfWeekExpr::Nth(day, nth) => {
                Self(DaysOfWeekKind::Nth, (u8::from(nth) << 3) | u8::from(day))
            }
            parse::DayOfWeekExpr::Many(exprs) => Self(
                DaysOfWeekKind::Pattern,
                exprs.into_iter().fold(0, Self::add_ors),
            ),
        }
    }
    #[inline]
    fn contains(&self, dt: DateTime<Utc>) -> bool {
        self.contains_date(dt.date())
    }
}
impl DaysOfWeek {
    const BITS: u8 = 8;
    const DAY_BITS: u8 = 0b0111_1111;
    const ONE_DAY_BITS: u8 = 0b0000_0111;
    const UPPER_BIT_BOUND: u8 = Self::DAY_BITS.trailing_ones() as u8;

    #[inline]
    fn kind(&self) -> DaysOfWeekKind {
        self.0
    }

    fn is_star(&self) -> bool {
        matches!(self.kind(), DaysOfWeekKind::Star)
    }

    #[inline]
    fn byte_to_weekday(value: u8) -> Weekday {
        match value {
            0 => Weekday::Sun,
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn last(&self) -> Option<Weekday> {
        if self.kind() == DaysOfWeekKind::Last {
            Some(Self::byte_to_weekday(self.1))
        } else {
            None
        }
    }

    #[inline]
    fn nth(&self) -> Option<(u8, Weekday)> {
        if let Self(DaysOfWeekKind::Nth, values) = *self {
            let weekday = values & Self::ONE_DAY_BITS;
            let nth = values >> 3;
            Some((nth, Self::byte_to_weekday(weekday)))
        } else {
            None
        }
    }

    #[inline]
    fn contains_date(&self, d: Date<Utc>) -> bool {
        match *self {
            Self(DaysOfWeekKind::Pattern, pattern) => {
                let mask = 1u8 << d.weekday().num_days_from_sunday();
                pattern & mask != 0
            }
            Self(DaysOfWeekKind::Nth, bits) => {
                let weekday = bits & Self::ONE_DAY_BITS;
                let nth = bits >> 3;
                let current_weekday = d.weekday().num_days_from_sunday() as u8;

                weekday == current_weekday && (d.day0() / 7) + 1 == nth as u32
            }
            Self(DaysOfWeekKind::Last, weekday) => {
                let current_weekday = d.weekday().num_days_from_sunday() as u8;
                weekday == current_weekday && d.day() + 7 > days_in_month(d)
            }
            _ => true,
        }
    }

    #[inline]
    fn value_pattern<T>(value: T) -> u8
    where
        T: Into<u8>,
    {
        let pattern = 1 << value.into();

        debug_assert_pattern!(pattern, Self::DAY_BITS);

        pattern
    }

    #[inline]
    fn add_ors(mut pattern: u8, expr: OrsExpr<parse::DayOfWeek>) -> u8 {
        match expr.normalize() {
            OrsExpr::One(one) => pattern |= Self::value_pattern(one),
            OrsExpr::Range(start, end) => {
                if start <= end {
                    let start = u8::from(start);
                    let end = u8::from(end);

                    // example: MON-FRI (or 2-6) (true value: 1-5)
                    // our bit map goes in reverse, so for weekdays
                    // our final mask should look like this
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   0   1   1   1   1   1   0
                    //
                    // to start with, our mask looks like this
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   1   1   1   1   1
                    let mut bits = Self::DAY_BITS;
                    // remove the end bits by shifting the bits to the right
                    // by the start value (1), then shift it back.
                    //
                    // shift right by 1
                    //                                 truncated
                    // ... ALL SAT FRI THU WED TUE MON SUN | (OOB)
                    // ... 0   0   1   1   1   1   1   1   | 1
                    //
                    // shift left by 1
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   1   1   1   1   0
                    bits = (bits >> start) << start;
                    // remove the start bits in the same way, shift the bits
                    // to the left by the number of bits in the integer (8) minus
                    // the end value (5) minus 1 (8 - 5 - 1 = 2).
                    // if we had a value that took up the whole bit map with a range
                    // that reached the max value, this operation would result in -1.
                    // In that case, we'd floor to 0 and not shift at all. but because
                    // it's the max value, we don't actually need to shift to truncate at
                    // all. so we can just skip this in that case.
                    //
                    // shift left by 2
                    // truncated
                    // (OOB)   | ALL SAT FRI THU WED TUE MON SUN
                    // 0   1   | 1   1   1   1   1   0   0   0
                    //
                    // shift right by 2
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   0   1   1   1   1   1   0
                    if end < Self::UPPER_BIT_BOUND {
                        // this won't overflow, so we might as well use wrapping arithmetic anyway
                        let end_shift = Self::BITS.wrapping_sub(end + 1);
                        bits = (bits << end_shift) >> end_shift;
                    }

                    debug_assert_pattern!(bits, Self::DAY_BITS);

                    pattern |= bits;
                } else {
                    // example : FRI-SUN (6-0)
                    // to match up with quartz schedulers, we have to support wrapping
                    // around, so for example with this expression, FRI,SAT,SUN,
                    // which should look like this:
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   0   0   0   0   1
                    //
                    // we remove bits from the middle a bit differently
                    // instead of like we do above. we have to make two
                    // masks which are missing either the left or right side
                    // and then OR those together.
                    //
                    // same as before, our first mask starts like this
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   1   1   1   1   1
                    let mut top_bits = Self::DAY_BITS;
                    // to remove the bottom bits, shift the top bits to the right
                    // by the start value (6) minus one (5), then shift back.
                    //
                    // shift right by 5
                    //                                 truncated
                    // ... ALL SAT FRI THU WED TUE MON SUN | (OOB)
                    // ... 0   0   0   0   0   0   1   1   | 1   ...
                    //
                    // shift left by 5
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   0   0   0   0   0
                    let start = u8::from(start) - 1;
                    top_bits = (top_bits >> start) << start;

                    // make a separate mask
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   1   1   1   1   1   1   1
                    let mut bottom_bits = Self::DAY_BITS;
                    // to remove the top bits, shift the top bits to the left
                    // by the number of bits in the integer (8) minus the end
                    // value (0) plus one (8 - 0 + 1 = 7)
                    //
                    // shift left by 7
                    // truncated
                    // ... (OOB)  | Out of mask bounds  ...
                    // ... 1   1  | 1   0   0   0   0   ...
                    //
                    //
                    // shift right by 7
                    //
                    // ... ALL SAT FRI THU WED TUE MON SUN
                    // ... 0   0   0   0   0   0   0   1
                    let end = u8::from(end) + 1;
                    let shift = Self::BITS.wrapping_sub(end);
                    bottom_bits = (bottom_bits << shift) >> shift;

                    let bits = top_bits | bottom_bits;

                    debug_assert_pattern!(bits, Self::DAY_BITS);

                    pattern |= bits;
                }
            }
            OrsExpr::Step { start, end, step } => {
                let start = u8::from(start);
                let end = u8::from(end);
                if start <= end {
                    let range = (start..=end).step_by(u8::from(step) as usize);

                    for shift in range {
                        pattern |= Self::value_pattern(shift);
                    }
                } else {
                    let back = start..=parse::DayOfWeek::MAX;
                    let front = parse::DayOfWeek::MIN..=end;
                    let range = back.chain(front).step_by(u8::from(step) as usize);

                    for shift in range {
                        pattern |= Self::value_pattern(shift);
                    }
                }
            }
        }
        pattern
    }
}

/// A bit-mask of all minutes in an hour set in a cron expression.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
struct Minutes(u64);
impl TimePattern for Minutes {
    type Expr = parse::Expr<parse::Minute>;

    #[inline]
    fn compile(expr: Self::Expr) -> Self {
        match expr {
            parse::Expr::All => Self(Self::ALL),
            parse::Expr::Many(exprs) => exprs.into_iter().fold(Self(0), Self::add_ors),
        }
    }

    /// Returns whether this mask contains the minute value 0-59
    #[inline]
    fn contains(&self, date: DateTime<Utc>) -> bool {
        let mask = 1u64 << date.minute();
        self.0 & mask != 0
    }
}
impl Minutes {
    const BITS: u8 = 64;
    const ALL: u64 = 0x0FFFFFFFFFFFFFFF;
    const UPPER_BIT_BOUND: u8 = Self::ALL.trailing_ones() as u8;

    #[inline]
    fn value_pattern<T>(value: T) -> u64
    where
        T: Into<u8>,
    {
        let pattern = 1 << value.into();

        debug_assert_pattern!(pattern, Self::ALL);

        pattern
    }

    #[inline]
    fn add_ors(mut self, expr: OrsExpr<parse::Minute>) -> Self {
        match expr.normalize() {
            OrsExpr::One(one) => self.0 |= Self::value_pattern(one),
            OrsExpr::Range(start, end) => {
                if start <= end {
                    let start = u8::from(start);
                    let end = u8::from(end);

                    // learn how this works in DayOfWeek's add_ors function
                    let mut bits = Self::ALL;
                    bits = (bits >> start) << start;
                    if end < Self::UPPER_BIT_BOUND {
                        let end_shift = Self::BITS.wrapping_sub(end + 1);
                        bits = (bits << end_shift) >> end_shift;
                    }
                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                } else {
                    let start = u8::from(start) - 1;
                    let end = u8::from(end) + 1;

                    let top_bits = (Self::ALL >> start) << start;

                    let bottom_shift = Self::BITS.wrapping_sub(end);
                    let bottom_bits = (Self::ALL << bottom_shift) >> bottom_shift;

                    let bits = top_bits | bottom_bits;

                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                }
            }
            OrsExpr::Step { start, end, step } => {
                let start = u8::from(start);
                let end = u8::from(end);
                if start <= end {
                    let range = (start..=end).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                } else {
                    let back = start..=parse::Minute::MAX;
                    let front = parse::Minute::MIN..=end;
                    let range = back.chain(front).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                }
            }
        }
        self
    }
}

/// A bit-mask of all hours in a day set in a cron expression.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
struct Hours(u32);
impl TimePattern for Hours {
    type Expr = parse::Expr<parse::Hour>;

    #[inline]
    fn compile(expr: Self::Expr) -> Self {
        match expr {
            parse::Expr::All => Self(Self::ALL),
            parse::Expr::Many(exprs) => exprs.into_iter().fold(Self(0), Self::add_ors),
        }
    }

    /// Returns whether this mask contains the hour value 0-23
    #[inline]
    fn contains(&self, dt: DateTime<Utc>) -> bool {
        self.contains_hour(dt.time())
    }
}
impl Hours {
    const BITS: u8 = 32;
    const ALL: u32 = 0x00FFFFFF;
    const UPPER_BIT_BOUND: u8 = Self::ALL.trailing_ones() as u8;

    #[inline]
    fn contains_hour(&self, time: NaiveTime) -> bool {
        let mask = 1u32 << time.hour();
        self.0 & mask != 0
    }

    #[inline]
    fn value_pattern<T>(value: T) -> u32
    where
        T: Into<u8>,
    {
        let pattern = 1 << value.into();

        debug_assert_pattern!(pattern, Self::ALL);

        pattern
    }

    #[inline]
    fn add_ors(mut self, expr: OrsExpr<parse::Hour>) -> Self {
        match expr.normalize() {
            OrsExpr::One(one) => self.0 |= Self::value_pattern(one),
            OrsExpr::Range(start, end) => {
                if start <= end {
                    let start = u8::from(start);
                    let end = u8::from(end);

                    // learn how this works in DayOfWeek's add_ors function
                    let mut bits = Self::ALL;
                    bits = (bits >> start) << start;
                    if end < Self::UPPER_BIT_BOUND {
                        let end_shift = Self::BITS.wrapping_sub(end + 1);
                        bits = (bits << end_shift) >> end_shift;
                    }
                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                } else {
                    let start = u8::from(start) - 1;
                    let end = u8::from(end) + 1;

                    let top_bits = (Self::ALL >> start) << start;

                    let bottom_shift = Self::BITS.wrapping_sub(end);
                    let bottom_bits = (Self::ALL << bottom_shift) >> bottom_shift;

                    let bits = top_bits | bottom_bits;

                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                }
            }
            OrsExpr::Step { start, end, step } => {
                let start = u8::from(start);
                let end = u8::from(end);
                if start <= end {
                    let range = (start..=end).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                } else {
                    let back = start..=parse::Hour::MAX;
                    let front = parse::Hour::MIN..=end;
                    let range = back.chain(front).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                }
            }
        }
        self
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum DaysOfMonthKind {
    Pattern,
    Star,
    Last,
    Weekday,
    LastWeekday,
}

/// A bit-mask of all the days of the month set in a cron expression.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct DaysOfMonth(DaysOfMonthKind, u32);
impl TimePattern for DaysOfMonth {
    type Expr = parse::DayOfMonthExpr;

    fn compile(expr: Self::Expr) -> Self {
        use parse::{DayOfMonthExpr, Last};
        match expr {
            DayOfMonthExpr::All => Self(DaysOfMonthKind::Star, 0),
            DayOfMonthExpr::Last(Last::Day) => Self(DaysOfMonthKind::Last, 0),
            DayOfMonthExpr::Last(Last::Weekday) => Self(DaysOfMonthKind::LastWeekday, 0),
            DayOfMonthExpr::Last(Last::Offset(offset)) => {
                Self(DaysOfMonthKind::Last, u8::from(offset) as u32)
            }
            DayOfMonthExpr::Last(Last::OffsetWeekday(offset)) => {
                Self(DaysOfMonthKind::LastWeekday, u8::from(offset) as u32)
            }
            DayOfMonthExpr::ClosestWeekday(day) => {
                Self(DaysOfMonthKind::Weekday, (u8::from(day) + 1) as u32)
            }
            DayOfMonthExpr::Many(exprs) => Self(
                DaysOfMonthKind::Pattern,
                exprs.into_iter().fold(0, Self::add_ors),
            ),
        }
    }

    #[inline]
    fn contains(&self, dt: DateTime<Utc>) -> bool {
        self.contains_date(dt.date())
    }
}
impl DaysOfMonth {
    const BITS: u8 = 32;
    const DAY_BITS: u32 = 0x0_7F_FF_FF_FF;
    const ONE_DAY_BITS: u32 = 0b0001_1111;
    const UPPER_BIT_BOUND: u8 = Self::DAY_BITS.trailing_ones() as u8;

    #[inline]
    fn kind(&self) -> DaysOfMonthKind {
        self.0
    }

    #[inline]
    fn is_last(&self) -> bool {
        matches!(
            self.kind(),
            DaysOfMonthKind::Last | DaysOfMonthKind::LastWeekday
        )
    }

    fn is_star(&self) -> bool {
        matches!(self.kind(), DaysOfMonthKind::Star)
    }

    /// Returns the one day set in this expression. Used to get last day offsets and the day
    /// in a closest weekday expression
    #[inline]
    fn one_value(&self) -> u8 {
        (self.1 & Self::ONE_DAY_BITS) as u8
    }

    #[inline]
    fn first_set0(&self) -> Option<u8> {
        let trailing = self.1.trailing_zeros() as u8;
        if trailing < Self::BITS {
            Some(trailing)
        } else {
            None
        }
    }

    #[inline]
    fn first_set(&self) -> Option<u8> {
        self.first_set0().map(|i| i + 1)
    }

    #[inline]
    fn contains_date(&self, date: Date<Utc>) -> bool {
        let is_weekend = |weekday| matches!(weekday, Weekday::Sat | Weekday::Sun);
        let is_weekday = |weekday| !is_weekend(weekday);

        let days_in_month = days_in_month(date);
        let day = date.day();

        match self {
            Self(DaysOfMonthKind::Pattern, pattern) => {
                let mask = 1u32 << (day - 1);
                pattern & mask != 0
            }
            Self(DaysOfMonthKind::Last, 0) => {
                // 'L'
                day == days_in_month
            }
            &Self(DaysOfMonthKind::Last, offset) => {
                // 'L' with an offset
                // Example: 'L-3'
                // Add to the day instead of subtracting from the days in the month,
                // since we allow an offset of 30, but the days in the month could be less
                // resulting in underflow.
                day + offset == days_in_month
            }
            Self(DaysOfMonthKind::LastWeekday, 0) => {
                // 'LW'
                let weekday = date.weekday();
                (is_weekday(weekday) && day == days_in_month)
                    || (weekday == Weekday::Fri && days_in_month - day < 3)
            }
            &Self(DaysOfMonthKind::LastWeekday, offset) => {
                // 'L' with an offset with the nearest weekday.
                // Example: 'L-3W'
                let weekday = date.weekday();
                let day_offsetted = day + offset;
                (is_weekday(weekday) && day_offsetted == days_in_month)
                    // don't check for weekend month ends since we're always offset by one
                    // at least, so our "end" can't be on a weekend ending month
                    // but do check if the month starts with a weekend and this is that weekend's
                    // Saturday or Sunday
                    || (weekday == Weekday::Mon && day_offsetted - days_in_month < 3)
                    || (weekday == Weekday::Fri && day_offsetted + 1 == days_in_month)
            }
            &Self(DaysOfMonthKind::Weekday, expected_day) => {
                let weekday = date.weekday();
                (is_weekday(weekday) && day == expected_day)
                    || (weekday == Weekday::Mon && day - 1 == expected_day)
                    // check for 1W on months where the 1st is Saturday and the 2nd is Sunday
                    || (weekday == Weekday::Mon && day == 3 && expected_day == 1)
                    || (weekday == Weekday::Fri && day + 1 == expected_day)
                    // check for 31W, 30W, 29W, 28W where they're the last day of the month and are on Sunday
                    || (weekday == Weekday::Fri && day + 2 == expected_day && expected_day == days_in_month)
            }
            _ => true,
        }
    }

    #[inline]
    fn value_pattern<T>(value: T) -> u32
    where
        T: Into<u8>,
    {
        let pattern = 1 << value.into();

        debug_assert_pattern!(pattern, Self::DAY_BITS);

        pattern
    }

    #[inline]
    fn add_ors(mut pattern: u32, expr: OrsExpr<parse::DayOfMonth>) -> u32 {
        match expr.normalize() {
            OrsExpr::One(day) => pattern |= Self::value_pattern(day),
            OrsExpr::Range(start, end) => {
                if start <= end {
                    let start = u8::from(start);
                    let end = u8::from(end);

                    // learn how this works in DayOfWeek's add_ors function
                    let mut bits = Self::DAY_BITS;
                    bits = (bits >> start) << start;
                    if end < Self::UPPER_BIT_BOUND {
                        let end_shift = Self::BITS.wrapping_sub(end + 1);
                        bits = (bits << end_shift) >> end_shift;
                    }
                    debug_assert_pattern!(bits, Self::DAY_BITS);

                    pattern |= bits;
                } else {
                    let start = u8::from(start) - 1;
                    let end = u8::from(end) + 1;

                    let top_bits = (Self::DAY_BITS >> start) << start;

                    let bottom_shift = Self::BITS.wrapping_sub(end);
                    let bottom_bits = (Self::DAY_BITS << bottom_shift) >> bottom_shift;

                    let bits = top_bits | bottom_bits;

                    debug_assert_pattern!(bits, Self::DAY_BITS);

                    pattern |= bits;
                }
            }
            OrsExpr::Step { start, end, step } => {
                let start = u8::from(start);
                let end = u8::from(end);
                if start <= end {
                    let range = (start..=end).step_by(u8::from(step) as usize);

                    for shift in range {
                        pattern |= Self::value_pattern(shift);
                    }
                } else {
                    let back = start..=parse::DayOfMonth::MAX;
                    let front = parse::DayOfMonth::MIN..=end;
                    let range = back.chain(front).step_by(u8::from(step) as usize);

                    for shift in range {
                        pattern |= Self::value_pattern(shift);
                    }
                }
            }
        }
        pattern
    }
}

/// A bit-mask of all the months set in a cron expression.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct Months(u16);
impl TimePattern for Months {
    type Expr = parse::Expr<parse::Month>;

    #[inline]
    fn compile(expr: Self::Expr) -> Self {
        match expr {
            parse::Expr::All => Self(Self::ALL),
            parse::Expr::Many(exprs) => exprs.into_iter().fold(Self(0), Self::add_ors),
        }
    }

    /// Returns whether this mask contains the month value 0-11
    #[inline]
    fn contains(&self, date: DateTime<Utc>) -> bool {
        self.contains_month(date.date())
    }
}
impl Months {
    const BITS: u8 = 16;
    const ALL: u16 = 0x0FFF;
    const UPPER_BIT_BOUND: u8 = Self::ALL.trailing_ones() as u8;

    #[inline]
    fn contains_month(&self, date: Date<Utc>) -> bool {
        let mask = 1u16 << date.month0();
        self.0 & mask != 0
    }

    #[inline]
    fn value_pattern<T>(value: T) -> u16
    where
        T: Into<u8>,
    {
        let pattern = 1 << value.into();

        debug_assert_pattern!(pattern, Self::ALL);

        pattern
    }

    #[inline]
    fn add_ors(mut self, expr: OrsExpr<parse::Month>) -> Self {
        match expr.normalize() {
            OrsExpr::One(one) => self.0 |= Self::value_pattern(one),
            OrsExpr::Range(start, end) => {
                if start <= end {
                    let start = u8::from(start);
                    let end = u8::from(end);

                    // learn how this works in DayOfWeek's add_ors function
                    let mut bits = Self::ALL;
                    bits = (bits >> start) << start;
                    if end < Self::UPPER_BIT_BOUND {
                        let end_shift = Self::BITS.wrapping_sub(end + 1);
                        bits = (bits << end_shift) >> end_shift;
                    }
                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                } else {
                    let start = u8::from(start) - 1;
                    let end = u8::from(end) + 1;

                    let top_bits = (Self::ALL >> start) << start;

                    let bottom_shift = Self::BITS.wrapping_sub(end);
                    let bottom_bits = (Self::ALL << bottom_shift) >> bottom_shift;

                    let bits = top_bits | bottom_bits;

                    debug_assert_pattern!(bits, Self::ALL);

                    self.0 |= bits;
                }
            }
            OrsExpr::Step { start, end, step } => {
                let start = u8::from(start);
                let end = u8::from(end);
                if start <= end {
                    let range = (start..=end).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                } else {
                    let back = start..=parse::Month::MAX;
                    let front = parse::Month::MIN..=end;
                    let range = back.chain(front).step_by(u8::from(step) as usize);

                    for shift in range {
                        self.0 |= Self::value_pattern(shift);
                    }
                }
            }
        }
        self
    }
}

/// A cron value. This can be used to iterate over all future matching times or quickly check if
/// a given time matches.
///
/// # Example
/// ```
/// use cfron::Cron;
/// use chrono::prelude::*;
///
/// let cron: Cron = "*/10 0 * OCT MON".parse().expect("Couldn't parse expression!");
///
/// // check if a given time is contained in an expression
/// assert!(cron.contains(Utc.ymd(2020, 10, 19).and_hms(0, 30, 0)));
///
/// // iterate over all future matching times
/// for time in cron.clone().iter_from(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0)).take(5) {
///     // Prints
///     // 1970-10-05 00:00:00 UTC
///     // 1970-10-05 00:10:00 UTC
///     // 1970-10-05 00:20:00 UTC
///     // 1970-10-05 00:30:00 UTC
///     // 1970-10-05 00:40:00 UTC
///     println!("{}", time);
///     assert!(cron.contains(time));
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Cron {
    minutes: Minutes,
    hours: Hours,
    dom: DaysOfMonth,
    months: Months,
    dow: DaysOfWeek,
}

impl FromStr for Cron {
    type Err = parse::CronParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // parse and compile
        // Any parsed expression can have redundant info, but we can
        // easily compress it into a neat bit map where each of the bits
        // of an integer represent the minutes/hours/days/months/weekdays
        // in a cron expression. It might be compressable further but I
        // doubt we'll need to do that.
        s.parse().map(Cron::new)
    }
}

impl Cron {
    /// Simplifies the cron expression into a cron value.
    pub fn new(expr: CronExpr) -> Self {
        Self {
            minutes: TimePattern::compile(expr.minutes),
            hours: TimePattern::compile(expr.hours),
            dom: TimePattern::compile(expr.doms),
            months: TimePattern::compile(expr.months),
            dow: TimePattern::compile(expr.dows),
        }
    }

    /// Returns whether this cron value will ever match any giving time.
    ///
    /// Some values can never match any given time. If an value matches
    /// for a day of the month that's beyond any of the valid days of the months matched
    /// then the value can never match.
    ///
    /// # Example
    /// ```
    /// use cfron::Cron;
    ///
    /// // Does have any since February has a 29th day on leap years
    /// assert!("* * 29 2 *".parse::<Cron>().unwrap().any());
    ///
    /// // Does not have any since November does not have a 31st day
    /// assert!(!"* * 31 11 *".parse::<Cron>().unwrap().any());
    /// ```
    #[inline]
    pub fn any(&self) -> bool {
        if self.dow.is_star() {
            if self.dom.is_star() {
                return true;
            }

            let first_set = if self.dom.is_last() {
                match self.dom.one_value() {
                    0 => return true,
                    offset => offset + 1,
                }
            } else {
                self.dom
                    .first_set()
                    .expect("At least one day should be set")
            };

            const MAX_31_MONTHS: u16 = 0b1010_1101_0101;
            const MAX_30_MONTHS: u16 = 0b0101_0010_1000;
            let max = if (self.months.0 & MAX_31_MONTHS) != 0 {
                31
            } else if (self.months.0 & MAX_30_MONTHS) != 0 {
                30
            } else {
                29
            };

            first_set <= max
        } else {
            true
        }
    }

    /// Returns whether this cron value matches the given time.
    /// # Example
    /// ```
    /// use cfron::Cron;
    /// use chrono::prelude::*;
    ///
    /// let cron: Cron = "*/10 0 * OCT MON".parse().expect("Couldn't parse expression!");
    ///
    /// // check if a given time is contained in an expression
    /// assert!(cron.contains(Utc.ymd(2020, 10, 19).and_hms(0, 30, 0)));
    /// ```
    #[inline]
    pub fn contains(&self, dt: DateTime<Utc>) -> bool {
        let contains_minutes_hour_months =
            self.minutes.contains(dt) && self.hours.contains(dt) && self.months.contains(dt);

        if !contains_minutes_hour_months {
            return false;
        }

        match (self.dom.is_star(), self.dow.is_star()) {
            (true, true) => true,
            (true, false) => self.dow.contains(dt),
            (false, true) => self.dom.contains(dt),
            (false, false) => self.dow.contains(dt) || self.dom.contains(dt),
        }
    }

    #[inline]
    fn contains_date(&self, date: Date<Utc>) -> bool {
        if !self.months.contains_month(date) {
            return false;
        }

        match (self.dom.is_star(), self.dow.is_star()) {
            (true, true) => true,
            (true, false) => self.dow.contains_date(date),
            (false, true) => self.dom.contains_date(date),
            (false, false) => self.dow.contains_date(date) || self.dom.contains_date(date),
        }
    }

    /// Creates an iterator of date times that match with the cron value.
    ///
    /// # Example
    /// ```
    /// use cfron::Cron;
    /// use chrono::prelude::*;
    ///
    /// let cron = "*/10 * * * *".parse::<Cron>().expect("Couldn't parse expression!");
    /// for time in cron.iter_from(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0)).take(5) {
    ///     // Prints
    ///     // 1970-01-01 00:00:00 UTC
    ///     // 1970-01-01 00:10:00 UTC
    ///     // 1970-01-01 00:20:00 UTC
    ///     // 1970-01-01 00:30:00 UTC
    ///     // 1970-01-01 00:40:00 UTC
    ///     println!("{}", time)
    /// }
    /// ```
    #[inline]
    pub fn iter_from(self, start: DateTime<Utc>) -> CronTimesIter {
        let start = start.trunc_subsecs(0).with_second(0).unwrap();
        let current = if self.any() {
            IterTime::Start(start)
        } else {
            IterTime::End
        };

        CronTimesIter {
            cron: self,
            current,
        }
    }

    /// Creates an iterator of date times that match with the cron value after the given date.
    ///
    /// # Example
    /// ```
    /// use cfron::Cron;
    /// use chrono::prelude::*;
    ///
    /// let cron = "*/10 * * * *".parse::<Cron>().expect("Couldn't parse expression!");
    /// for time in cron.iter_after(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0)).take(5) {
    ///     // Prints
    ///     // 1970-01-01 00:10:00 UTC
    ///     // 1970-01-01 00:20:00 UTC
    ///     // 1970-01-01 00:30:00 UTC
    ///     // 1970-01-01 00:40:00 UTC
    ///     // 1970-01-01 00:50:00 UTC
    ///     println!("{}", time)
    /// }
    /// ```
    #[inline]
    pub fn iter_after(self, start: DateTime<Utc>) -> CronTimesIter {
        let start = start.trunc_subsecs(0).with_second(0).unwrap();
        let current = if self.any() {
            IterTime::Next(start)
        } else {
            IterTime::End
        };

        CronTimesIter {
            cron: self,
            current,
        }
    }

    /// Returns the next time the cron will match including the given date.
    ///
    /// # Example
    /// ```
    /// use cfron::Cron;
    /// use chrono::prelude::*;
    ///
    /// let cron = "*/10 * * * *".parse::<Cron>().expect("Couldn't parse expression!");
    /// let date = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
    /// // the given date matches the expression, so we get the same date back (truncated)
    /// assert_eq!(cron.next_from(date), Some(date));
    /// ```
    #[inline]
    pub fn next_from(&self, date: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let date = date.trunc_subsecs(0).with_second(0).unwrap();
        if !self.any() {
            return None;
        }

        if self.contains(date) {
            Some(date)
        } else {
            self.find_next(date)
        }
    }

    /// Returns the next time the cron will match after the given date.
    ///
    /// # Example
    /// ```
    /// use cfron::Cron;
    /// use chrono::prelude::*;
    ///
    /// let cron = "*/10 * * * *".parse::<Cron>().expect("Couldn't parse expression!");
    /// let date = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
    /// assert_eq!(cron.next_after(date), date.with_minute(10));
    /// ```
    #[inline]
    pub fn next_after(&self, date: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let date = date.trunc_subsecs(0).with_second(0).unwrap();
        if self.any() {
            self.find_next(date)
        } else {
            None
        }
    }

    const MUST_CONTAIN_MINUTE: &'static str = "Expression must contain at least one minute";

    fn next_minute(time: NaiveTime) -> Option<NaiveTime> {
        if time.minute() < 59 {
            Some(NaiveTime::from_hms(time.hour(), time.minute() + 1, 0))
        } else if time.hour() < 23 {
            Some(NaiveTime::from_hms(time.hour() + 1, 0, 0))
        } else {
            None
        }
    }

    fn find_next(&self, dt: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if self.contains_date(dt.date()) {
            if let Some(next_minute) = Self::next_minute(dt.time()) {
                if let Some(time) = self.find_next_time(next_minute) {
                    return dt.date().and_time(time);
                }
            }
        }

        let tomorrow = dt.date().succ_opt()?;
        let time = NaiveTime::from_hms(0, 0, 0);

        if let Some(next_date) = self.find_next_date(tomorrow) {
            let time = self.find_next_time(time).expect(Self::MUST_CONTAIN_MINUTE);

            return next_date.and_time(time);
        }

        let mut year = tomorrow.year().checked_add(1)?;
        loop {
            let next_year = Utc.ymd_opt(year, 1, 1).single()?;
            if let Some(next_date) = self.find_next_date(next_year) {
                let time = self.find_next_time(time).expect(Self::MUST_CONTAIN_MINUTE);

                return next_date.and_time(time);
            } else {
                year = year.checked_add(1)?;
            }
        }
    }

    /// Gets the next minute (current inclusive) matching the cron expression, or none if the current
    /// minute / no upcoming minute in the hour matches. The current minute is a value 0-59.
    fn find_next_minute(&self, current_minute: u32) -> Option<u32> {
        let Minutes(map) = self.minutes;
        // clear the minutes we're already past
        let bottom_cleared = (map >> current_minute) << current_minute;
        // count trailing zeroes to find the first set. if none is set, we get back the number of
        // bits in the integer
        let trailing_zeroes = bottom_cleared.trailing_zeros();
        if trailing_zeroes < Minutes::BITS as u32 {
            Some(trailing_zeroes)
        } else {
            None
        }
    }

    /// Gets the next matching (current inclusive) hour in the cron expression. The returned matching
    /// hour is a value 0-23.
    fn find_next_hour(&self, current_hour: u32) -> Option<u32> {
        let Hours(map) = self.hours;
        let bottom_cleared = (map >> current_hour) << current_hour;
        let trailing_zeroes = bottom_cleared.trailing_zeros();
        if trailing_zeroes < Hours::BITS as u32 {
            Some(trailing_zeroes)
        } else {
            None
        }
    }

    /// Gets the next matching (current inclusive) day of the month or day of the week that
    /// matches the cron expression. The returned matching day is a value 0-30.
    fn find_next_day(&self, date: Date<Utc>) -> Option<u32> {
        match (self.dom.is_star(), self.dow.is_star()) {
            (true, true) => Some(date.day0()),
            (true, false) => self.find_next_weekday(date),
            (false, true) => self.find_next_day_of_month(date),
            (false, false) => {
                let next_weekday = self.find_next_weekday(date);
                let next_day = self.find_next_day_of_month(date);
                match (next_day, next_weekday) {
                    (Some(day), Some(weekday)) => Some(cmp::min(day, weekday)),
                    (Some(day), None) => Some(day),
                    (None, Some(day)) => Some(day),
                    (None, None) => None,
                }
            }
        }
    }

    /// Gets the next matching (current inclusive) day of the month that matches the cron expression.
    /// The returned matching day is a value 0-30.
    fn find_next_day_of_month(&self, date: Date<Utc>) -> Option<u32> {
        let days_in_month = days_in_month(date);
        match self.dom.kind() {
            DaysOfMonthKind::Last => match self.dom.one_value() {
                // 'L'
                0 => Some(days_in_month - 1),
                // 'L-3'
                offset => {
                    let expected = (days_in_month - 1).checked_sub(offset as u32)?;
                    if expected < date.day0() {
                        None
                    } else {
                        Some(expected)
                    }
                }
            },
            DaysOfMonthKind::LastWeekday => match self.dom.one_value() {
                // 'LW'
                0 => {
                    let days_in_month = days_in_month - 1;
                    let real_day = match Self::weekday_for_day(days_in_month, date) {
                        Weekday::Sat => days_in_month - 1,
                        Weekday::Sun => days_in_month - 2,
                        _ => days_in_month,
                    };
                    if real_day < date.day0() {
                        None
                    } else {
                        Some(real_day)
                    }
                }
                // 'L-3W'
                offset => {
                    let days_in_month = days_in_month - 1;
                    let last_in_month =
                        days_in_month.checked_sub(offset as u32).map(|new_day| {
                            match Self::weekday_for_day(new_day, date) {
                                Weekday::Sat => {
                                    if new_day == 0 {
                                        2
                                    } else {
                                        new_day - 1
                                    }
                                }
                                Weekday::Sun => new_day + 1,
                                _ => new_day,
                            }
                        })?;
                    if last_in_month < date.day0() {
                        None
                    } else {
                        Some(last_in_month)
                    }
                }
            },
            DaysOfMonthKind::Weekday => {
                let days_in_month = days_in_month - 1;
                let expected_day = (self.dom.one_value() - 1) as u32;
                let real_day = match Self::weekday_for_day(expected_day, date) {
                    Weekday::Sat => {
                        if expected_day == 0 {
                            2
                        } else {
                            expected_day - 1
                        }
                    }
                    Weekday::Sun => {
                        if expected_day == days_in_month {
                            days_in_month - 2
                        } else {
                            expected_day + 1
                        }
                    }
                    _ => expected_day,
                };

                if real_day >= date.day0() && real_day <= days_in_month {
                    Some(real_day)
                } else {
                    None
                }
            }
            _ => {
                let current_day = date.day0();
                let map = self.dom.1 & DaysOfMonth::DAY_BITS;
                let bottom_cleared = (map >> current_day) << current_day;
                let trailing_zeroes = bottom_cleared.trailing_zeros();
                if trailing_zeroes < days_in_month {
                    Some(trailing_zeroes)
                } else {
                    None
                }
            }
        }
    }

    /// Gets the next matching (current inclusive) day of the week that matches the cron expression.
    /// The returned matching day is a value 0-30.
    fn find_next_weekday(&self, date: Date<Utc>) -> Option<u32> {
        let days_in_month = days_in_month(date);
        match self.dow.kind() {
            DaysOfWeekKind::Last => {
                let cron_weekday = self.dow.last().unwrap().num_days_from_sunday();
                let current_weekday = date.weekday().num_days_from_sunday();
                // calculate an offset that can be added to the current day to get what would be a day
                // of a week where that day is the expected weekday for the cron
                let weekday_offset = if cron_weekday < current_weekday {
                    // example:
                    // current: Thursday, expected: Tuesday
                    // 7 - (4 - 2) = 5
                    // October 0th 2020 (Thursday) + 5 = October 5th 2020 (Tuesday)
                    7 - (current_weekday - cron_weekday)
                } else {
                    // example:
                    // expected: Thursday, current: Tuesday
                    // (4 - 2) = 2
                    // October 5th 2020 (Tuesday) + 2 = October 7th 2020 (Thursday)
                    cron_weekday - current_weekday
                };
                // the remainder of 7 can be used with day0 to determine the first day0 of the
                // current day of the week in the month. it doesn't matter if this calculation
                // overflows the date out of the month (31st + 5 = 36th) since we're just looking
                // for the first day.
                let first_week_day = (date.day0() + weekday_offset) % 7;
                // using that we can find the last day this weekday occurs in the month
                let last_day = match (days_in_month, first_week_day) {
                    // special 5 week weekday handling
                    (29, day @ 0)
                    | (30, day @ 0)
                    | (30, day @ 1)
                    | (31, day @ 0)
                    | (31, day @ 1)
                    | (31, day @ 2) => day + (7 * 4),
                    (_, day) => day + (7 * 3),
                };

                if date.day0() <= last_day {
                    Some(last_day)
                } else {
                    None
                }
            }
            DaysOfWeekKind::Nth => {
                let (nth, day) = self.dow.nth().unwrap();
                let cron_weekday = day.num_days_from_sunday();
                let current_weekday = date.weekday().num_days_from_sunday();
                let weekday_offset = if cron_weekday < current_weekday {
                    7 - (current_weekday - cron_weekday)
                } else {
                    cron_weekday - current_weekday
                };
                let first_week_day = (date.day0() + weekday_offset) % 7;
                let nth_day = first_week_day + (7 * (nth - 1) as u32);
                if nth_day < days_in_month && nth_day >= date.day0() {
                    Some(nth_day)
                } else {
                    None
                }
            }
            DaysOfWeekKind::Pattern => {
                let current_weekday = date.weekday().num_days_from_sunday();
                let map = self.dow.1 & DaysOfWeek::DAY_BITS;
                let bottom_cleared = (map >> current_weekday) << current_weekday;
                let trailing_zeroes = bottom_cleared.trailing_zeros();
                let next_day = if trailing_zeroes < DaysOfWeek::BITS as u32 {
                    date.day0() + (trailing_zeroes - current_weekday)
                } else {
                    let next_week = map.trailing_zeros();
                    let remaining_days = (6 - current_weekday) + 1;
                    date.day0() + remaining_days + next_week
                };
                if next_day < days_in_month {
                    Some(next_day)
                } else {
                    None
                }
            }
            _ => Some(date.day0()),
        }
    }

    /// Gets the weekday for a given day of the month using the current date for the month
    fn weekday_for_day(day: u32, date: Date<Utc>) -> Weekday {
        let expected_first_day = (day % 7) as usize;
        let first_day_of_current_weekday = (date.day0() % 7) as usize;
        let current_weekday = date.weekday().num_days_from_sunday() as usize;

        // I'm bad at math, so here's all the answers instead
        use Weekday::*;
        const TABLE: [[[Weekday; 7]; 7]; 7] = [
            [
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
            ],
            [
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
            ],
            [
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
            ],
            [
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
            ],
            [
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
            ],
            [
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
            ],
            [
                [Mon, Tue, Wed, Thu, Fri, Sat, Sun],
                [Tue, Wed, Thu, Fri, Sat, Sun, Mon],
                [Wed, Thu, Fri, Sat, Sun, Mon, Tue],
                [Thu, Fri, Sat, Sun, Mon, Tue, Wed],
                [Fri, Sat, Sun, Mon, Tue, Wed, Thu],
                [Sat, Sun, Mon, Tue, Wed, Thu, Fri],
                [Sun, Mon, Tue, Wed, Thu, Fri, Sat],
            ],
        ];

        TABLE[first_day_of_current_weekday][expected_first_day][current_weekday]
    }

    /// Gets the next matching (current inclusive) month that matches the cron expression.
    /// The returned matching month is a value 0-11.
    fn find_next_month(&self, current_month: u32) -> Option<u32> {
        let Months(map) = self.months;
        let bottom_cleared = (map >> current_month) << current_month;
        let trailing_zeroes = bottom_cleared.trailing_zeros();
        if trailing_zeroes < Months::BITS as u32 {
            Some(trailing_zeroes)
        } else {
            None
        }
    }

    fn find_next_time(&self, time: NaiveTime) -> Option<NaiveTime> {
        let hour = time.hour();
        if self.hours.contains_hour(time) {
            if let Some(next_minute) = self.find_next_minute(time.minute()) {
                return Some(NaiveTime::from_hms(hour, next_minute, 0));
            }
        }

        if hour < 23 {
            if let Some(next_hour) = self.find_next_hour(hour + 1) {
                let minute = self.find_next_minute(0).expect(Self::MUST_CONTAIN_MINUTE);

                return Some(NaiveTime::from_hms(next_hour, minute, 0));
            }
        }

        None
    }

    fn find_next_date(&self, date: Date<Utc>) -> Option<Date<Utc>> {
        const VALID_NEXT_DAY: &str = "Day should be valid for giving month";
        const VALID_NEXT_MONTH: &str = "Month should be valid";

        let mut month = date.month();
        if self.months.contains_month(date) {
            if let Some(next_day) = self.find_next_day(date) {
                return Some(date.with_day0(next_day).expect(VALID_NEXT_DAY));
            }
        }

        while month <= 11 {
            if let Some(next_month) = self.find_next_month(month) {
                month = next_month;

                let start_month_date = date
                    .with_day(1)
                    .expect(VALID_NEXT_DAY)
                    .with_month0(month)
                    .expect(VALID_NEXT_MONTH);
                if let Some(month_day) = self.find_next_day(start_month_date) {
                    return Some(start_month_date.with_day0(month_day).expect(VALID_NEXT_DAY));
                } else {
                    month += 1;
                }
            } else {
                break;
            }
        }

        None
    }
}

enum IterTime {
    /// Return the current time if it matches, otherwise change to `Next` and get the next time
    Start(DateTime<Utc>),
    /// Return the next time that matches the cron expression after this one
    Next(DateTime<Utc>),
    /// The end of the iterator
    End,
}

/// An iterator over the times matching the contained cron value.
/// Created with [`Cron::iter_from`] and [`Cron::iter_after`].
///
/// [`Cron::iter_from`]: struct.Cron.html#method.iter_from
/// [`Cron::iter_after`]: struct.Cron.html#method.iter_after
pub struct CronTimesIter {
    cron: Cron,
    current: IterTime,
}

impl CronTimesIter {
    /// Returns the underlying cron value.
    pub fn cron(&self) -> &Cron {
        &self.cron
    }
}

impl Iterator for CronTimesIter {
    type Item = DateTime<Utc>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            IterTime::Start(time) => {
                self.current = IterTime::Next(time);
                if self.cron.contains(time) {
                    Some(time)
                } else {
                    self.next() // call back in and get the next item
                }
            }
            IterTime::Next(time) => match self.cron.find_next(time) {
                Some(date) => {
                    self.current = IterTime::Next(date);
                    Some(date)
                }
                None => {
                    self.current = IterTime::End;
                    None
                }
            },
            IterTime::End => None,
        }
    }
}

impl core::iter::FusedIterator for CronTimesIter {}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_does_contain(cron: &str, dates: impl IntoIterator<Item = impl AsRef<str>>) {
        let parsed: Cron = cron.parse().unwrap();

        for date in dates
            .into_iter()
            .map(|s| s.as_ref().parse::<DateTime<Utc>>().unwrap())
        {
            assert!(
                parsed.contains(date),
                "Cron \"{}\" should contain {}. Compiled: {:#?}",
                cron,
                date,
                parsed
            );
        }
    }

    fn check_does_not_contain(cron: &str, dates: impl IntoIterator<Item = impl AsRef<str>>) {
        let parsed: Cron = cron.parse().unwrap();

        for date in dates
            .into_iter()
            .map(|s| s.as_ref().parse::<DateTime<Utc>>().unwrap())
        {
            assert!(
                !parsed.contains(date),
                "Cron \"{}\" shouldn't contain {}. Compiled {:#?}",
                cron,
                date,
                parsed
            );
        }
    }

    #[test]
    fn parse_check_anytime() {
        check_does_contain(
            "* * * * *",
            &[
                "1970-01-1T00:00:00+00:00",
                "2016-11-08T23:53:57+00:00",
                "2020-07-04T15:42:30+00:00",
                "2072-02-29T01:15:23+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_specific_time() {
        let cron = "5 0 23 8 *";

        check_does_contain(
            cron,
            &["2020-08-23T00:05:00+00:00", "2020-08-23T00:05:30+00:00"],
        );

        check_does_not_contain(
            cron,
            &[
                "1970-01-1T00:00:00+00:00",
                "2016-11-08T23:53:57+00:00",
                "2020-07-04T15:42:30+00:00",
                "2072-02-29T01:15:23+00:00",
                "2020-08-23T11:05:00+00:00",
            ],
        );
    }

    /// check to make sure we don't accidentally include any off-by-one errors with ranges
    #[test]
    fn parse_check_specific_time_as_ranges() {
        let cron = "5-5 0-0 23-23 8-8 *";

        check_does_contain(
            cron,
            &["2020-08-23T00:05:00+00:00", "2020-08-23T00:05:30+00:00"],
        );

        check_does_not_contain(
            cron,
            &[
                "1970-01-01T00:00:00+00:00",
                "2016-11-08T23:53:57+00:00",
                "2020-07-04T15:42:30+00:00",
                "2072-02-29T01:15:23+00:00",
                "2020-08-23T11:05:00+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_overflow_time_ranges() {
        // The 31st and 1st of January and December,
        // at 11:00 PM, 11:59 PM, 12:00 AM, and 12:59 AM
        let cron = "59-0 23-0 31-1 12-1 *";

        check_does_contain(
            cron,
            &[
                "2020-01-31T00:59:00+00:00",
                "2020-01-31T00:00:00+00:00",
                "2020-01-31T23:59:00+00:00",
                "2020-01-31T23:00:00+00:00",
                "2020-01-01T00:59:00+00:00",
                "2020-01-01T00:00:00+00:00",
                "2020-01-01T23:59:00+00:00",
                "2020-01-01T23:00:00+00:00",
                "2020-12-31T00:59:00+00:00",
                "2020-12-31T00:00:00+00:00",
                "2020-12-31T23:59:00+00:00",
                "2020-12-31T23:00:00+00:00",
                "2020-12-01T00:59:00+00:00",
                "2020-12-01T00:00:00+00:00",
                "2020-12-01T23:59:00+00:00",
                "2020-12-01T23:00:00+00:00",
            ],
        );

        // Midnight on every Saturday and Sunday in January
        let cron = "0 0 * JAN SAT-SUN";

        check_does_contain(
            cron,
            &[
                "2020-01-04T00:00:00+00:00",
                "2020-01-05T00:00:00+00:00",
                "2020-01-11T00:00:00+00:00",
                "2020-01-12T00:00:00+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_limits() {
        let cron = "0,59 0,23 1,31 1,12 *";

        check_does_contain(
            cron,
            &[
                "2020-01-01T00:00:00+00:00",
                "2020-01-01T00:59:00+00:00",
                "2020-01-01T23:59:00+00:00",
                "2020-01-31T23:59:00+00:00",
                "2020-12-31T23:59:00+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_anytime_but_its_ranges() {
        let cron = "0-59 0-23 1-31 1-12 *";

        check_does_contain(
            cron,
            &[
                "1970-01-1T00:00:00+00:00",
                "2016-11-08T23:53:57+00:00",
                "2020-07-04T15:42:30+00:00",
                "2072-02-29T01:15:23+00:00",
            ],
        );

        let cron = "0-59 0-23 * 1-12 1-7";

        check_does_contain(
            cron,
            &[
                "1970-01-1T00:00:00+00:00",
                "2016-11-08T23:53:57+00:00",
                "2020-07-04T15:42:30+00:00",
                "2072-02-29T01:15:23+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_leap_days() {
        let cron = "0 0 L FEB *";

        check_does_contain(
            cron,
            &[
                "2400-02-29T00:00:00+00:00",
                "2300-02-28T00:00:00+00:00",
                "2200-02-28T00:00:00+00:00",
                "2100-02-28T00:00:00+00:00",
                "2024-02-29T00:00:00+00:00",
                "2020-02-29T00:00:00+00:00",
                "2004-02-29T00:00:00+00:00",
                "2000-02-29T00:00:00+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_offset_leap_days() {
        let cron = "0 0 L-1 FEB *";

        check_does_contain(
            cron,
            &[
                "2400-02-28T00:00:00+00:00",
                "2300-02-27T00:00:00+00:00",
                "2200-02-27T00:00:00+00:00",
                "2100-02-27T00:00:00+00:00",
                "2024-02-28T00:00:00+00:00",
                "2020-02-28T00:00:00+00:00",
                "2004-02-28T00:00:00+00:00",
                "2000-02-28T00:00:00+00:00",
            ],
        );

        check_does_not_contain(
            cron,
            &[
                "2400-02-29T00:00:00+00:00",
                "2300-02-28T00:00:00+00:00",
                "2200-02-28T00:00:00+00:00",
                "2100-02-28T00:00:00+00:00",
                "2024-02-29T00:00:00+00:00",
                "2020-02-29T00:00:00+00:00",
                "2004-02-29T00:00:00+00:00",
                "2000-02-29T00:00:00+00:00",
            ],
        );
    }

    #[test]
    fn parse_check_offset_weekend_start_months() {
        let cron = "0 0 L-30W * *";

        check_does_contain(
            cron,
            &["2021-05-3T00:00:00+00:00", "2022-01-3T00:00:00+00:00"],
        );
    }

    #[test]
    fn parse_check_offset_weekend_start_months_beyond_days() {
        let cron = "0 0 L-28W FEB *";

        check_does_not_contain(
            cron,
            &["2021-05-3T00:00:00+00:00", "2022-01-3T00:00:00+00:00"],
        );
    }

    #[test]
    fn parse_check_last_weekdays() {
        let cron = "0 0 LW MAY *";

        check_does_contain(
            cron,
            &[
                "2025-05-30T00:00:00+00:00", // Last day is a Saturday
                "2021-05-31T00:00:00+00:00", // Last day is a Monday
                "2020-05-29T00:00:00+00:00", // Last day is a Sunday
            ],
        );
    }

    #[test]
    fn parse_check_last_weekdays_offset() {
        let cron = "0 0 L-1W MAY *";

        check_does_contain(
            cron,
            &[
                "2025-05-30T00:00:00+00:00", // Offset last day is a Friday
                "2021-05-31T00:00:00+00:00", // Offset last day is a Sunday
                "2020-05-29T00:00:00+00:00", // Offset last day is a Saturday
            ],
        );
    }

    #[test]
    fn parse_check_closest_weekday() {
        let cron = "0 0 1W MAY *";

        check_does_contain(
            cron,
            &[
                "2020-05-01T00:00:00+00:00", // First day is a Friday
                "2022-05-02T00:00:00+00:00", // First day is a Sunday
                "2021-05-03T00:00:00+00:00", // First day is a Saturday
            ],
        )
    }

    #[test]
    fn parse_check_last_weekday() {
        let cron = "0 0 * * 7L"; // the last saturday of every month

        check_does_contain(
            cron,
            &[
                "2020-01-25T00:00:00+00:00",
                "2020-02-29T00:00:00+00:00",
                "2020-03-28T00:00:00+00:00",
                "2020-04-25T00:00:00+00:00",
                "2020-05-30T00:00:00+00:00",
            ],
        );

        check_does_not_contain(
            cron,
            &[
                "2020-01-31T00:00:00+00:00",
                "2020-02-28T00:00:00+00:00",
                "2020-03-31T00:00:00+00:00",
                "2020-04-30T00:00:00+00:00",
                "2020-05-31T00:00:00+00:00",
            ],
        )
    }

    #[test]
    fn parse_check_nth_weekday() {
        let cron = "0 0 * * SAT#5"; // the 5th saturday of every month

        check_does_contain(
            cron,
            &[
                "2020-02-29T00:00:00+00:00",
                "2020-05-30T00:00:00+00:00",
                "2020-08-29T00:00:00+00:00",
                "2020-10-31T00:00:00+00:00",
            ],
        );

        check_does_not_contain(
            cron,
            &[
                "2020-01-31T00:00:00+00:00",
                "2020-02-28T00:00:00+00:00",
                "2020-03-31T00:00:00+00:00",
                "2020-04-30T00:00:00+00:00",
                "2020-05-31T00:00:00+00:00",
            ],
        )
    }

    #[test]
    fn parse_check_steps() {
        // all the impls step impls follow the same code, so i'll just test minutes for now
        let cron = "*/15,30-59/10 0 * * *";

        check_does_contain(
            cron,
            &[
                "2020-01-01T00:00:00+00:00",
                "2020-01-01T00:15:00+00:00",
                "2020-01-01T00:30:00+00:00",
                "2020-01-01T00:40:00+00:00",
                "2020-01-01T00:45:00+00:00",
                "2020-01-01T00:50:00+00:00",
            ],
        )
    }

    #[test]
    fn parse_check_overflow_range_step() {
        // previous code assumed the start was before the end
        let cron = "0 20-4/2 * * *";

        check_does_contain(
            cron,
            &[
                "2020-01-01T20:00:00+00:00",
                "2020-01-01T22:00:00+00:00",
                "2020-01-01T00:00:00+00:00",
                "2020-01-01T02:00:00+00:00",
                "2020-01-01T04:00:00+00:00",
            ],
        );
    }
}
