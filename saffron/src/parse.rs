//! A module allowing for inspection of a parsed cron expression. This can be used to
//! accurately describe an expression without reducing it into a cron value.

#[cfg(not(feature = "std"))]
use alloc::vec::{self, Vec};

use crate::internal::Sealed;
use core::cmp::Ordering;
use core::convert::TryFrom;
use core::fmt::{self, Display, Formatter};
use core::iter::{Chain, Once};
use core::marker::PhantomData;
use core::slice;
use core::str::FromStr;
use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{char, digit1, space1},
    combinator::{all_consuming, map, map_res, opt},
    sequence::tuple,
    IResult,
};

#[cfg(feature = "std")]
use std::vec;

pub use crate::describe::*;

/// An error returned if an expression type value is out of range.
#[derive(Debug)]
pub struct ValueOutOfRangeError;

impl Display for ValueOutOfRangeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        "The expression value is out range of valid values".fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValueOutOfRangeError {}

/// A trait implemented for expression values that defines a MIN value and a MAX value.
pub trait ExprValue: Sized + Sealed {
    /// The max value for an expression value
    const MAX: u8;
    /// The min value for an expression value
    const MIN: u8;

    /// The max value as this expression value type
    fn max() -> Self;
    /// The min value as this expression value type
    fn min() -> Self;
}

/// A minute value, 0-59
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Minute(u8);
impl Sealed for Minute {}
impl ExprValue for Minute {
    const MAX: u8 = 59;
    const MIN: u8 = 0;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<Minute> for u8 {
    /// Returns the value, 0-59
    #[inline]
    fn from(m: Minute) -> Self {
        m.0
    }
}
impl TryFrom<u8> for Minute {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for Minute {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}

/// An hour value, 0-23
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hour(u8);
impl Sealed for Hour {}
impl ExprValue for Hour {
    const MAX: u8 = 23;
    const MIN: u8 = 0;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<Hour> for u8 {
    #[inline]
    /// Returns the value, 0-23
    fn from(m: Hour) -> Self {
        m.0
    }
}
impl TryFrom<u8> for Hour {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for Hour {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}

/// A day of the month, 1-31
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DayOfMonth(u8);
impl Sealed for DayOfMonth {}
impl ExprValue for DayOfMonth {
    const MAX: u8 = 31;
    const MIN: u8 = 1;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<DayOfMonth> for u8 {
    #[inline]
    /// Returns the zero based day of the month, 0-30
    fn from(m: DayOfMonth) -> Self {
        m.0 - 1
    }
}
impl TryFrom<u8> for DayOfMonth {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for DayOfMonth {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}
/// A last day of the month offset, 1-30
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DayOfMonthOffset(u8);
impl Sealed for DayOfMonthOffset {}
impl ExprValue for DayOfMonthOffset {
    const MAX: u8 = 30;
    const MIN: u8 = 1;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<DayOfMonthOffset> for u8 {
    #[inline]
    /// Returns the zero based day of the month, 0-30
    fn from(m: DayOfMonthOffset) -> Self {
        m.0
    }
}
impl TryFrom<u8> for DayOfMonthOffset {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for DayOfMonthOffset {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}

/// A month, 1-12
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Month(u8);
impl Sealed for Month {}
impl ExprValue for Month {
    const MAX: u8 = 12;
    const MIN: u8 = 1;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<Month> for u8 {
    #[inline]
    /// Returns the zero based month, 0-11
    fn from(m: Month) -> Self {
        m.0 - 1
    }
}
impl From<chrono::Month> for Month {
    fn from(m: chrono::Month) -> Self {
        use chrono::Month::*;
        match m {
            January => Self(1),
            February => Self(2),
            March => Self(3),
            April => Self(4),
            May => Self(5),
            June => Self(6),
            July => Self(7),
            August => Self(8),
            September => Self(9),
            October => Self(10),
            November => Self(11),
            December => Self(12),
        }
    }
}
impl From<Month> for chrono::Month {
    fn from(Month(m): Month) -> chrono::Month {
        use chrono::Month::*;
        match m {
            1 => January,
            2 => February,
            3 => March,
            4 => April,
            5 => May,
            6 => June,
            7 => July,
            8 => August,
            9 => September,
            10 => October,
            11 => November,
            12 => December,
            _ => unreachable!(),
        }
    }
}
impl TryFrom<u8> for Month {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for Month {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}

/// An "nth" day, 1-5
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NthDay(u8);
impl Sealed for NthDay {}
impl ExprValue for NthDay {
    const MAX: u8 = 5;
    const MIN: u8 = 1;

    fn max() -> Self {
        Self(Self::MAX)
    }
    fn min() -> Self {
        Self(Self::MIN)
    }
}
impl From<NthDay> for u8 {
    #[inline]
    fn from(m: NthDay) -> Self {
        m.0
    }
}
impl TryFrom<u8> for NthDay {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}
impl PartialEq<u8> for NthDay {
    #[inline]
    fn eq(&self, other: &u8) -> bool {
        &self.0 == other
    }
}

/// A day of the week, 1-7 (Sun-Sat)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayOfWeek(chrono::Weekday);
impl Sealed for DayOfWeek {}
impl ExprValue for DayOfWeek {
    const MAX: u8 = 7;
    const MIN: u8 = 1;

    fn max() -> Self {
        Self(chrono::Weekday::Sat)
    }
    fn min() -> Self {
        Self(chrono::Weekday::Sun)
    }
}
impl PartialOrd for DayOfWeek {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0
            .number_from_sunday()
            .partial_cmp(&other.0.number_from_sunday())
    }
}
impl Ord for DayOfWeek {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .number_from_sunday()
            .cmp(&other.0.number_from_sunday())
    }
}
impl From<DayOfWeek> for u8 {
    #[inline]
    /// Returns the zero based day of the week, 0-6
    fn from(m: DayOfWeek) -> Self {
        m.0.num_days_from_sunday() as u8
    }
}
impl From<chrono::Weekday> for DayOfWeek {
    #[inline]
    fn from(w: chrono::Weekday) -> Self {
        Self(w)
    }
}
impl From<DayOfWeek> for chrono::Weekday {
    #[inline]
    fn from(DayOfWeek(w): DayOfWeek) -> Self {
        w
    }
}
impl TryFrom<u8> for DayOfWeek {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use chrono::Weekday::*;

        Ok(Self(match value {
            1 => Sun,
            2 => Mon,
            3 => Tue,
            4 => Wed,
            5 => Thu,
            6 => Fri,
            7 => Sat,
            _ => return Err(ValueOutOfRangeError),
        }))
    }
}
impl PartialEq<chrono::Weekday> for DayOfWeek {
    #[inline]
    fn eq(&self, other: &chrono::Weekday) -> bool {
        &self.0 == other
    }
}

/// A step value constrained by a expression value. The max value of this type differs depending
/// on the type `E`. The minimum value is always 1.
///
/// | Type           | Max |
/// | -------------- | --- |
/// | [`Minute`]     | 59  |
/// | [`Hour`]       | 23  |
/// | [`DayOfMonth`] | 30  |
/// | [`Month`]      | 11  |
/// | [`DayOfWeek`]  | 6   |
///
/// [`Minute`]: struct.Minute.html
/// [`Hour`]: struct.Hour.html
/// [`DayOfMonth`]: struct.DayOfMonth.html
/// [`Month`]: struct.Month.html
/// [`DayOfWeek`]: struct.DayOfWeek.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Step<E> {
    e: PhantomData<fn(E) -> E>,
    value: u8,
}
impl<E: Sealed> Sealed for Step<E> {}
impl<E: ExprValue> ExprValue for Step<E> {
    const MAX: u8 = E::MAX - E::MIN;
    // This assumes every MIN value is 0 or 1. If that changes this breaks and it's the
    // problem of whoever reads this. Hopefully the const-eval story of Rust is better
    // when you're fixing this
    const MIN: u8 = E::MIN | 1;

    fn max() -> Self {
        Self {
            e: PhantomData,
            value: Self::MAX,
        }
    }
    fn min() -> Self {
        Self {
            e: PhantomData,
            value: Self::MIN,
        }
    }
}
impl<E> From<Step<E>> for u8 {
    #[inline]
    fn from(s: Step<E>) -> Self {
        s.value
    }
}
impl<E: ExprValue> TryFrom<u8> for Step<E> {
    type Error = ValueOutOfRangeError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self {
                e: PhantomData,
                value,
            })
        } else {
            Err(ValueOutOfRangeError)
        }
    }
}

/// A day of the week expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DayOfWeekExpr {
    /// A '*' character
    All,
    /// A `L` character, the last day of the week for the month, paired with a value
    Last(DayOfWeek),
    /// A '#' character
    Nth(DayOfWeek, NthDay),
    /// Possibly multiple unique, ranges, or steps
    Many(Exprs<DayOfWeek>),
}

/// A "last" expression for [`DayOfMonthExpr`]
///
/// [`DayOfMonthExpr`]: enum.DayOfMonthExpr.html
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Last {
    /// An `L` expression. The last day of the month.
    Day,
    /// An `LW` expression. The last weekday of the month.
    Weekday,
    /// The last day of the month offsetted by a value.
    /// For example, a `L-3`, the 3rd to last day of the month
    Offset(DayOfMonthOffset),
    /// The closest weekday to the last day of the month offsetted by a value.
    /// For example, a `L-3W`, the weekday closest to the 3rd to last day of the month.
    OffsetWeekday(DayOfMonthOffset),
}

/// A day of the month expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DayOfMonthExpr {
    /// A '*' character
    All,
    /// An expression containing an 'L' character.
    Last(Last),
    /// A 'W' expression, used to mean the closest weekday to the specified day of the month
    ClosestWeekday(DayOfMonth),
    /// Possibly multiple unique, ranges, or steps
    Many(Exprs<DayOfMonth>),
}

/// A generic expression that can take a '*' or many exprs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expr<E> {
    /// A '*' character
    All,
    /// Possibly multiple unique, ranges, or steps
    Many(Exprs<E>),
}

/// Either one value, a range, or a step expression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OrsExpr<E> {
    /// One value
    One(E),
    /// A '-' character.
    Range(E, E),
    /// A '/' character.
    Step {
        /// The start value. If the start value is '*', this is the min value of E.
        start: E,
        /// The end value. If the step expression does not specify a value, this is the max value of E.
        end: E,
        /// The step value.
        step: Step<E>,
    },
}

impl<E: Copy + ExprValue + PartialEq> OrsExpr<E> {
    /// Normalizes the expression, simplifying it.
    ///
    /// Normalizations:
    ///  * A range of equal start and end points (i.e. 1-1) is simplified into one value (1)
    ///  * A step of equal start and end points (i.e. 1-1/3) is simplified into one value (1)
    ///  * A step where the start is equal to the max value of E (i.e. 59/3) is simplified into one value (59)
    ///  * A step where the step value is one (i.e. 5/1 or 5-30/1) is simplified into a range (5-59 or 5-30)
    pub fn normalize(self) -> OrsExpr<E> {
        match self {
            OrsExpr::Range(a, b)
            | OrsExpr::Step {
                start: a, end: b, ..
            } if a == b => OrsExpr::One(a),
            OrsExpr::Step { step, start, end } if u8::from(step) == 1 => OrsExpr::Range(start, end),
            x => x,
        }
    }
}

/// A set of expressions with at least one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exprs<E> {
    /// The first expression
    pub first: OrsExpr<E>,
    /// The rest of the other expressions in the set.
    pub tail: Vec<OrsExpr<E>>,
}

/// An immutable iterator over all expressions in a set of [`Exprs`]
///
/// [`Exprs`]: struct.Exprs.html
pub type ExprsIter<'a, E> = Chain<Once<&'a OrsExpr<E>>, slice::Iter<'a, OrsExpr<E>>>;

/// An owned iterator over all expressions in a set of [`Exprs`]
///
/// [`Exprs`]: struct.Exprs.html
pub type IntoExprsIter<E> = Chain<Once<OrsExpr<E>>, vec::IntoIter<OrsExpr<E>>>;

impl<E> Exprs<E> {
    /// Creates a new set of [`Exprs`] using the first given [`OrsExpr`]
    ///
    /// [`Exprs`]: struct.Exprs.html
    /// [`OrsExpr`]: enum.OrsExpr.html
    pub fn new(first: OrsExpr<E>) -> Self {
        Self {
            first,
            tail: Vec::new(),
        }
    }

    /// Iterates over all expressions in this set
    pub fn iter(&self) -> ExprsIter<E> {
        core::iter::once(&self.first).chain(self.tail.iter())
    }
}

impl<E> IntoIterator for Exprs<E> {
    type Item = OrsExpr<E>;
    type IntoIter = IntoExprsIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        core::iter::once(self.first).chain(self.tail.into_iter())
    }
}

impl<'a, E> IntoIterator for &'a Exprs<E> {
    type Item = &'a OrsExpr<E>;
    type IntoIter = ExprsIter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A parsed cron expression. This can be used to describe the expression or reduce it into a
/// [`Cron`](../struct.Cron.html) value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CronExpr {
    /// The minute part of the expression
    pub minutes: Expr<Minute>,
    /// The hour part of the expression
    pub hours: Expr<Hour>,
    /// The day of the month part of the expression
    pub doms: DayOfMonthExpr,
    /// The month part of the expression
    pub months: Expr<Month>,
    /// The day of the week part of the expression.
    pub dows: DayOfWeekExpr,
}

/// A formatter for displaying a cron expression description in a specified language
#[derive(Debug, Clone, Copy)]
pub struct LanguageFormatter<'a, L> {
    expr: &'a CronExpr,
    lang: L,
}

impl<'a, L: Language> Display for LanguageFormatter<'a, L> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.lang.fmt_expr(self.expr, f)
    }
}

impl CronExpr {
    /// Returns a formatter to display the cron expression in the provided language
    ///
    /// # Example
    /// ```
    /// use saffron::parse::{CronExpr, English};
    ///
    /// let cron: CronExpr = "* * * * *".parse().expect("Valid cron expression");
    ///
    /// let description = cron.describe(English::default()).to_string();
    /// assert_eq!("Every minute", description);
    /// ```
    pub fn describe<L: Language>(&self, lang: L) -> LanguageFormatter<L> {
        LanguageFormatter { expr: self, lang }
    }
}

/// An error indicating that the provided cron expression failed to parse
#[derive(Debug)]
pub struct CronParseError(());

impl Display for CronParseError {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        "Failed to parse cron expression".fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CronParseError {}

/// A parser that can parse a single value, a range of values, or a step expression
fn ors_expr<E, F>(f: F) -> impl Fn(&str) -> IResult<&str, OrsExpr<E>>
where
    E: ExprValue + TryFrom<u8, Error = ValueOutOfRangeError> + Ord + Copy,
    F: Fn(&str) -> IResult<&str, E>,
{
    move |input: &str| {
        let (input, value) = alt((&f, map(char('*'), |_| ExprValue::min())))(input)?;
        match opt(alt((char('/'), char('-'))))(input)? {
            (input, Some('/')) => map(step_digit::<E>(), |step| OrsExpr::Step {
                start: value,
                end: ExprValue::max(),
                step,
            })(input),
            (input, Some('-')) => {
                let (input, end) = f(input)?;
                match opt(char('/'))(input)? {
                    (input, Some(_)) => map(step_digit::<E>(), |step| OrsExpr::Step {
                        start: value,
                        end,
                        step,
                    })(input),
                    (input, None) => Ok((input, OrsExpr::Range(value, end))),
                }
            }
            (input, _) => Ok((input, OrsExpr::One(value))),
        }
    }
}

/// Consumes a set of trailing ORS expressions
fn tail_ors_exprs<'a, E, F>(
    mut input: &'a str,
    f: F,
    mut exprs: Exprs<E>,
) -> IResult<&'a str, Exprs<E>>
where
    E: ExprValue + TryFrom<u8, Error = ValueOutOfRangeError> + Ord + Copy,
    F: Fn(&str) -> IResult<&str, E>,
{
    loop {
        let comma = opt(char(','))(input)?;
        input = comma.0;
        if comma.1.is_none() {
            break Ok((input, exprs));
        }

        let expr = ors_expr::<E, _>(&f)(input)?;
        input = expr.0;
        exprs.tail.push(expr.1);
    }
}

/// A parser that can parse delimited expressions given a parser for that part.
/// This can't parse day of the month or week expressions.
fn expr<E, F>(f: F) -> impl Fn(&str) -> IResult<&str, Expr<E>>
where
    E: ExprValue + TryFrom<u8, Error = ValueOutOfRangeError> + Ord + Copy,
    F: Fn(&str) -> IResult<&str, E>,
{
    move |mut input: &str| {
        let expressions: Exprs<E>;
        // Attempt to read a `*`. If that succeeds,
        // try to read a `/` for a step expr.
        // If this isn't a step expr, return Expr::All,
        // If it's not a `*`, initialize the expressions
        // list with an ors_expr.
        let star = opt(char('*'))(input)?;
        input = star.0;
        if star.1.is_some() {
            let slash = opt(char('/'))(input)?;
            input = slash.0;
            // If there is no slash after this, just return All and expect the next
            // parser to fail if it's invalid
            if slash.1.is_none() {
                return Ok((input, Expr::All));
            }
            let step = step_digit::<E>()(input)?;
            input = step.0;
            expressions = Exprs::new(OrsExpr::Step {
                start: ExprValue::min(),
                end: ExprValue::max(),
                step: step.1,
            })
        } else {
            let expr = ors_expr::<E, _>(&f)(input)?;
            input = expr.0;
            expressions = Exprs::new(expr.1)
        }

        let (input, exprs) = tail_ors_exprs(input, &f, expressions)?;

        Ok((input, Expr::Many(exprs)))
    }
}

#[inline]
fn map_digit1<E>() -> impl Fn(&str) -> IResult<&str, E>
where
    E: ExprValue + TryFrom<u8, Error = ValueOutOfRangeError>,
{
    move |input: &str| {
        map_res(digit1, |s: &str| {
            let value = s
                .parse::<u8>()
                // discard error, we won't see it anyway
                .map_err(|_| ValueOutOfRangeError)?;

            E::try_from(value)
        })(input)
    }
}

#[inline]
fn step_digit<E>() -> impl Fn(&str) -> IResult<&str, Step<E>>
where
    E: ExprValue,
{
    map_digit1()
}

fn month(s: &str) -> IResult<&str, Month> {
    alt((
        map_digit1::<Month>(),
        map(tag_no_case("JAN"), |_| Month(1)),
        map(tag_no_case("FEB"), |_| Month(2)),
        map(tag_no_case("MAR"), |_| Month(3)),
        map(tag_no_case("APR"), |_| Month(4)),
        map(tag_no_case("MAY"), |_| Month(5)),
        map(tag_no_case("JUN"), |_| Month(6)),
        map(tag_no_case("JUL"), |_| Month(7)),
        map(tag_no_case("AUG"), |_| Month(8)),
        map(tag_no_case("SEP"), |_| Month(9)),
        map(tag_no_case("OCT"), |_| Month(10)),
        map(tag_no_case("NOV"), |_| Month(11)),
        map(tag_no_case("DEC"), |_| Month(12)),
    ))(s)
}

#[inline]
fn minutes_expr(s: &str) -> IResult<&str, Expr<Minute>> {
    expr(map_digit1())(s)
}

#[inline]
fn hours_expr(s: &str) -> IResult<&str, Expr<Hour>> {
    expr(map_digit1())(s)
}

fn dom_expr(input: &str) -> IResult<&str, DayOfMonthExpr> {
    let dom = map_digit1::<DayOfMonth>();

    let (input, start) = opt(alt((char('*'), char('L'))))(input)?;
    match start {
        Some('*') => {
            let (input, maybe_step) = opt(tuple((char('/'), step_digit::<DayOfMonth>())))(input)?;

            if let Some((_, step)) = maybe_step {
                let exprs = Exprs::new(OrsExpr::Step {
                    start: DayOfMonth(1),
                    end: ExprValue::max(),
                    step,
                });

                let (input, exprs) = tail_ors_exprs(input, dom, exprs)?;
                Ok((input, DayOfMonthExpr::Many(exprs)))
            } else {
                Ok((input, DayOfMonthExpr::All))
            }
        }
        Some('L') => {
            let (input, modifier) = opt(alt((char('-'), char('W'))))(input)?;
            match modifier {
                Some('-') => {
                    let offset = map_digit1::<DayOfMonthOffset>();
                    let (input, (offset, weekday)) = tuple((offset, opt(char('W'))))(input)?;

                    if weekday.is_some() {
                        Ok((input, DayOfMonthExpr::Last(Last::OffsetWeekday(offset))))
                    } else {
                        Ok((input, DayOfMonthExpr::Last(Last::Offset(offset))))
                    }
                }
                Some('W') => Ok((input, DayOfMonthExpr::Last(Last::Weekday))),
                _ => Ok((input, DayOfMonthExpr::Last(Last::Day))),
            }
        }
        _ => {
            let (input, day) = dom(input)?;

            let (input, maybe_char) = opt(alt((char('W'), char('-'), char('/'))))(input)?;
            match maybe_char {
                Some('W') => Ok((input, DayOfMonthExpr::ClosestWeekday(day))),
                Some('-') => {
                    let (input, (end, slash)) = tuple((&dom, opt(char('/'))))(input)?;

                    let (input, exprs) = if slash.is_none() {
                        (input, Exprs::new(OrsExpr::Range(day, end)))
                    } else {
                        let (input, step) = step_digit::<DayOfMonth>()(input)?;
                        (
                            input,
                            Exprs::new(OrsExpr::Step {
                                start: day,
                                end,
                                step,
                            }),
                        )
                    };

                    let (input, exprs) = tail_ors_exprs(input, dom, exprs)?;
                    Ok((input, DayOfMonthExpr::Many(exprs)))
                }
                Some('/') => {
                    let (input, step) = step_digit::<DayOfMonth>()(input)?;
                    let exprs = Exprs::new(OrsExpr::Step {
                        start: day,
                        end: ExprValue::max(),
                        step,
                    });

                    let (input, exprs) = tail_ors_exprs(input, dom, exprs)?;
                    Ok((input, DayOfMonthExpr::Many(exprs)))
                }
                _ => {
                    let (input, exprs) = tail_ors_exprs(input, dom, Exprs::new(OrsExpr::One(day)))?;
                    Ok((input, DayOfMonthExpr::Many(exprs)))
                }
            }
        }
    }
}

#[inline]
fn months_expr(s: &str) -> IResult<&str, Expr<Month>> {
    expr(month)(s)
}

fn dow_expr(input: &str) -> IResult<&str, DayOfWeekExpr> {
    fn dow(s: &str) -> IResult<&str, DayOfWeek> {
        alt((
            map_digit1::<DayOfWeek>(),
            map(tag_no_case("SUN"), |_| DayOfWeek(chrono::Weekday::Sun)),
            map(tag_no_case("MON"), |_| DayOfWeek(chrono::Weekday::Mon)),
            map(tag_no_case("TUE"), |_| DayOfWeek(chrono::Weekday::Tue)),
            map(tag_no_case("WED"), |_| DayOfWeek(chrono::Weekday::Wed)),
            map(tag_no_case("THU"), |_| DayOfWeek(chrono::Weekday::Thu)),
            map(tag_no_case("FRI"), |_| DayOfWeek(chrono::Weekday::Fri)),
            map(tag_no_case("SAT"), |_| DayOfWeek(chrono::Weekday::Sat)),
        ))(s)
    }

    let (input, start) = opt(alt((char('*'), char('L'))))(input)?;

    match start {
        Some('*') => {
            let (input, maybe_step) = opt(tuple((char('/'), step_digit::<DayOfWeek>())))(input)?;
            if let Some((_, step)) = maybe_step {
                let exprs = Exprs::new(OrsExpr::Step {
                    start: DayOfWeek(chrono::Weekday::Sun),
                    end: ExprValue::max(),
                    step,
                });

                let (input, exprs) = tail_ors_exprs(input, dow, exprs)?;
                Ok((input, DayOfWeekExpr::Many(exprs)))
            } else {
                Ok((input, DayOfWeekExpr::All))
            }
        }
        Some('L') => Ok((
            input,
            DayOfWeekExpr::Many(Exprs::new(OrsExpr::One(DayOfWeek(chrono::Weekday::Sat)))),
        )),
        _ => {
            let (input, day) = dow(input)?;
            let (input, maybe_char) =
                opt(alt((char('L'), char('#'), char('-'), char('/'))))(input)?;

            match maybe_char {
                Some('L') => Ok((input, DayOfWeekExpr::Last(day))),
                Some('#') => map(map_digit1::<NthDay>(), move |nth| {
                    DayOfWeekExpr::Nth(day, nth)
                })(input),
                Some('-') => {
                    let (input, (end, slash)) = tuple((&dow, opt(char('/'))))(input)?;

                    let (input, exprs) = if slash.is_none() {
                        (input, Exprs::new(OrsExpr::Range(day, end)))
                    } else {
                        let (input, step) = step_digit::<DayOfWeek>()(input)?;
                        (
                            input,
                            Exprs::new(OrsExpr::Step {
                                start: day,
                                end,
                                step,
                            }),
                        )
                    };

                    let (input, exprs) = tail_ors_exprs(input, dow, exprs)?;
                    Ok((input, DayOfWeekExpr::Many(exprs)))
                }
                Some('/') => {
                    let (input, step) = step_digit::<DayOfWeek>()(input)?;
                    let exprs = Exprs::new(OrsExpr::Step {
                        start: day,
                        end: ExprValue::max(),
                        step,
                    });

                    let (input, exprs) = tail_ors_exprs(input, dow, exprs)?;
                    Ok((input, DayOfWeekExpr::Many(exprs)))
                }
                _ => {
                    let (input, exprs) = tail_ors_exprs(input, dow, Exprs::new(OrsExpr::One(day)))?;
                    Ok((input, DayOfWeekExpr::Many(exprs)))
                }
            }
        }
    }
}

impl FromStr for CronExpr {
    type Err = CronParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, expr) = all_consuming(map(
            tuple((
                minutes_expr,
                space1,
                hours_expr,
                space1,
                dom_expr,
                space1,
                months_expr,
                space1,
                dow_expr,
            )),
            |(minutes, _, hours, _, doms, _, months, _, dows)| CronExpr {
                minutes,
                hours,
                doms,
                months,
                dows,
            },
        ))(s)
        .map_err(|_| CronParseError(()))?;

        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use core::fmt::Debug;

    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use super::*;

    fn exprs<E, I>(iter: I) -> Exprs<E>
    where
        I: IntoIterator<Item = OrsExpr<E>>,
    {
        let mut iter = iter.into_iter();
        let first = iter.next().expect("Iterator must have at least one item");
        let tail = iter.collect();
        Exprs { first, tail }
    }

    fn e<E>(value: u8) -> E
    where
        E: TryFrom<u8>,
        E::Error: Debug,
    {
        E::try_from(value).unwrap()
    }

    fn o<E>(value: u8) -> OrsExpr<E>
    where
        E: TryFrom<u8>,
        E::Error: Debug,
    {
        OrsExpr::One(e(value))
    }

    fn r<E>(start: u8, end: u8) -> OrsExpr<E>
    where
        E: TryFrom<u8>,
        E::Error: Debug,
    {
        let start = e(start);
        let end = e(end);
        OrsExpr::Range(start, end)
    }

    fn s<E>(value: u8, step: u8) -> OrsExpr<E>
    where
        E: TryFrom<u8> + ExprValue,
        E::Error: Debug,
    {
        let start = e(value);
        let step = e(step);
        OrsExpr::Step {
            start,
            end: E::max(),
            step,
        }
    }

    fn rs<E>(start: u8, end: u8, step: u8) -> OrsExpr<E>
    where
        E: TryFrom<u8> + ExprValue,
        E::Error: Debug,
    {
        let start = e(start);
        let end = e(end);
        let step = e(step);
        OrsExpr::Step { start, end, step }
    }

    mod minutes {
        use super::*;

        #[test]
        fn all() {
            assert_eq!(minutes_expr("*"), Ok(("", Expr::All)))
        }

        #[test]
        fn only_match_first_star() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(minutes_expr("*,*"), Ok((",*", Expr::All)))
        }

        #[test]
        fn star_step() {
            assert_eq!(
                minutes_expr("*/5"),
                Ok(("", Expr::Many(exprs(vec![s(0, 5)]))))
            )
        }

        #[test]
        fn multi_star_step() {
            assert_eq!(
                minutes_expr("*/5,*/3"),
                Ok(("", Expr::Many(exprs(vec![s(0, 5), s(0, 3)]))))
            )
        }

        #[test]
        fn star_range_doesnt_make_sense() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(minutes_expr("*-30/5,*/3"), Ok(("-30/5,*/3", Expr::All)))
        }

        #[test]
        fn one_value() {
            assert_eq!(minutes_expr("0"), Ok(("", Expr::Many(exprs(vec![o(0)])))))
        }

        #[test]
        fn many_one_value() {
            assert_eq!(
                minutes_expr("5,15,25,35,45,55"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![o(5), o(15), o(25), o(35), o(45), o(55)]))
                ))
            )
        }

        #[test]
        fn one_range() {
            assert_eq!(
                minutes_expr("0-30"),
                Ok(("", Expr::Many(exprs(vec![r(0, 30)]))))
            )
        }

        #[test]
        fn overflow_range() {
            assert_eq!(
                minutes_expr("50-10"),
                Ok(("", Expr::Many(exprs(vec![r(50, 10)]))))
            )
        }

        #[test]
        fn many_range() {
            assert_eq!(
                minutes_expr("0-5,10-15,20-25,30-35,40-45,50-55"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![
                        r(0, 5),
                        r(10, 15),
                        r(20, 25),
                        r(30, 35),
                        r(40, 45),
                        r(50, 55)
                    ]))
                ))
            )
        }

        #[test]
        fn step() {
            assert_eq!(
                minutes_expr("0/5"),
                Ok(("", Expr::Many(exprs(vec![s(0, 5)]))))
            )
        }

        #[test]
        fn step_with_star_step() {
            assert_eq!(
                minutes_expr("1/3,*/5"),
                Ok(("", Expr::Many(exprs(vec![s(1, 3), s(0, 5)]))))
            )
        }

        #[test]
        fn many_steps() {
            assert_eq!(
                minutes_expr("1/3,2/3,5/10"),
                Ok(("", Expr::Many(exprs(vec![s(1, 3), s(2, 3), s(5, 10)]))))
            )
        }

        #[test]
        fn range_step() {
            assert_eq!(
                minutes_expr("0-30/5"),
                Ok(("", Expr::Many(exprs(vec![rs(0, 30, 5)]))))
            )
        }

        #[test]
        fn many_range_step() {
            assert_eq!(
                minutes_expr("0-30/5,30-59/3"),
                Ok(("", Expr::Many(exprs(vec![rs(0, 30, 5), rs(30, 59, 3)]))))
            )
        }

        #[test]
        fn values_ranges_steps_and_ranges() {
            assert_eq!(
                minutes_expr("0,5-10,10-30/3,30/3"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![o(0), r(5, 10), rs(10, 30, 3), s(30, 3)]))
                ))
            )
        }

        #[test]
        fn limits() {
            assert!(matches!(minutes_expr("60"), Err(_)));
            assert!(matches!(minutes_expr("0-60"), Err(_)));
            // a step greater than the max value is not allowed (since it doesn't make sense)
            assert!(matches!(minutes_expr("0/60"), Err(_)));
            assert!(matches!(minutes_expr("0-60/5"), Err(_)));
            // a step of 0 is not allowed (since it doesn't make sense)
            assert!(matches!(minutes_expr("0/0"), Err(_)));
            assert!(matches!(minutes_expr("0-59/0"), Err(_)));
        }
    }

    mod hours {
        use super::*;

        #[test]
        fn all() {
            assert_eq!(hours_expr("*"), Ok(("", Expr::All)))
        }

        #[test]
        fn only_match_first_star() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(hours_expr("*,*"), Ok((",*", Expr::All)))
        }

        #[test]
        fn star_step() {
            assert_eq!(
                hours_expr("*/3"),
                Ok(("", Expr::Many(exprs(vec![s(0, 3)]))))
            )
        }

        #[test]
        fn multi_star_step() {
            assert_eq!(
                hours_expr("*/3,*/4"),
                Ok(("", Expr::Many(exprs(vec![s(0, 3), s(0, 4)]))))
            )
        }

        #[test]
        fn star_range_doesnt_make_sense() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(hours_expr("*-6/3,*/4"), Ok(("-6/3,*/4", Expr::All)))
        }

        #[test]
        fn one_value() {
            assert_eq!(hours_expr("0"), Ok(("", Expr::Many(exprs(vec![o(0)])))))
        }

        #[test]
        fn many_one_value() {
            assert_eq!(
                hours_expr("0,3,6,9,12,15,18,21"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![
                        o(0),
                        o(3),
                        o(6),
                        o(9),
                        o(12),
                        o(15),
                        o(18),
                        o(21)
                    ]))
                ))
            )
        }

        #[test]
        fn one_range() {
            assert_eq!(
                hours_expr("0-12"),
                Ok(("", Expr::Many(exprs(vec![r(0, 12)]))))
            )
        }

        #[test]
        fn overflow_range() {
            assert_eq!(
                hours_expr("22-2"),
                Ok(("", Expr::Many(exprs(vec![r(22, 2)]))))
            )
        }

        #[test]
        fn many_range() {
            assert_eq!(
                hours_expr("0-3,6-9,12-15,18-21"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![r(0, 3), r(6, 9), r(12, 15), r(18, 21)]))
                ))
            )
        }

        #[test]
        fn step() {
            assert_eq!(
                hours_expr("0/3"),
                Ok(("", Expr::Many(exprs(vec![s(0, 3)]))))
            )
        }

        #[test]
        fn step_with_star_step() {
            assert_eq!(
                hours_expr("1/2,*/4"),
                Ok(("", Expr::Many(exprs(vec![s(1, 2), s(0, 4)]))))
            )
        }

        #[test]
        fn many_steps() {
            assert_eq!(
                hours_expr("1/2,2/3,3/4"),
                Ok(("", Expr::Many(exprs(vec![s(1, 2), s(2, 3), s(3, 4)]))))
            )
        }

        #[test]
        fn range_step() {
            assert_eq!(
                hours_expr("0-12/4"),
                Ok(("", Expr::Many(exprs(vec![rs(0, 12, 4)]))))
            )
        }

        #[test]
        fn many_range_step() {
            assert_eq!(
                hours_expr("0-12/4,12-23/3"),
                Ok(("", Expr::Many(exprs(vec![rs(0, 12, 4), rs(12, 23, 3)]))))
            )
        }

        #[test]
        fn values_ranges_steps_and_ranges() {
            assert_eq!(
                hours_expr("0,0-6/3,6-12,12/3"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![o(0), rs(0, 6, 3), r(6, 12), s(12, 3)]))
                ))
            )
        }

        #[test]
        fn limits() {
            assert!(matches!(hours_expr("24"), Err(_)));
            assert!(matches!(hours_expr("0-24"), Err(_)));
            // a step greater than the max value is not allowed (since it doesn't make sense)
            assert!(matches!(hours_expr("0/24"), Err(_)));
            assert!(matches!(hours_expr("0-24/2"), Err(_)));
            // a step of 0 is not allowed (since it doesn't make sense)
            assert!(matches!(hours_expr("0/0"), Err(_)));
            assert!(matches!(hours_expr("0-23/0"), Err(_)));
        }
    }

    mod months {
        use super::*;

        #[test]
        fn all() {
            assert_eq!(months_expr("*"), Ok(("", Expr::All)))
        }

        #[test]
        fn only_match_first_star() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(months_expr("*,*"), Ok((",*", Expr::All)))
        }

        #[test]
        fn star_step() {
            assert_eq!(
                months_expr("*/3"),
                Ok(("", Expr::Many(exprs(vec![s(1, 3)]))))
            )
        }

        #[test]
        fn multi_star_step() {
            assert_eq!(
                months_expr("*/3,*/4"),
                Ok(("", Expr::Many(exprs(vec![s(1, 3), s(1, 4)]))))
            )
        }

        #[test]
        fn star_range_doesnt_make_sense() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(months_expr("*-6/3,*/4"), Ok(("-6/3,*/4", Expr::All)))
        }

        #[test]
        fn one_value() {
            assert_eq!(months_expr("1"), Ok(("", Expr::Many(exprs(vec![o(1)])))))
        }

        #[test]
        fn word_values() {
            // caps
            assert_eq!(months_expr("JAN"), Ok(("", Expr::Many(exprs(vec![o(1)])))));
            assert_eq!(months_expr("FEB"), Ok(("", Expr::Many(exprs(vec![o(2)])))));
            assert_eq!(months_expr("MAR"), Ok(("", Expr::Many(exprs(vec![o(3)])))));
            assert_eq!(months_expr("APR"), Ok(("", Expr::Many(exprs(vec![o(4)])))));

            // lower
            assert_eq!(months_expr("may"), Ok(("", Expr::Many(exprs(vec![o(5)])))));
            assert_eq!(months_expr("jun"), Ok(("", Expr::Many(exprs(vec![o(6)])))));
            assert_eq!(months_expr("jul"), Ok(("", Expr::Many(exprs(vec![o(7)])))));
            assert_eq!(months_expr("aug"), Ok(("", Expr::Many(exprs(vec![o(8)])))));

            // mixed
            assert_eq!(months_expr("sEp"), Ok(("", Expr::Many(exprs(vec![o(9)])))));
            assert_eq!(months_expr("ocT"), Ok(("", Expr::Many(exprs(vec![o(10)])))));
            assert_eq!(months_expr("NOv"), Ok(("", Expr::Many(exprs(vec![o(11)])))));
            assert_eq!(months_expr("Dec"), Ok(("", Expr::Many(exprs(vec![o(12)])))));
        }

        #[test]
        fn many_one_value() {
            assert_eq!(
                months_expr("1,MAR,6,SEP,12"),
                Ok(("", Expr::Many(exprs(vec![o(1), o(3), o(6), o(9), o(12)]))))
            )
        }

        #[test]
        fn one_range() {
            assert_eq!(
                months_expr("1-12"),
                Ok(("", Expr::Many(exprs(vec![r(1, 12)]))))
            );
            assert_eq!(
                months_expr("JAN-DEC"),
                Ok(("", Expr::Many(exprs(vec![r(1, 12)]))))
            )
        }

        #[test]
        fn overflow_range() {
            assert_eq!(
                months_expr("11-FEB"),
                Ok(("", Expr::Many(exprs(vec![r(11, 2)]))))
            );
            assert_eq!(
                months_expr("NOV-2"),
                Ok(("", Expr::Many(exprs(vec![r(11, 2)]))))
            )
        }

        #[test]
        fn many_range() {
            assert_eq!(
                months_expr("1-MAR,MAY-7,SEP-11"),
                Ok(("", Expr::Many(exprs(vec![r(1, 3), r(5, 7), r(9, 11)]))))
            )
        }

        #[test]
        fn step() {
            assert_eq!(
                months_expr("1/3"),
                Ok(("", Expr::Many(exprs(vec![s(1, 3)]))))
            )
        }

        #[test]
        fn step_with_star_step() {
            assert_eq!(
                months_expr("2/2,*/4"),
                Ok(("", Expr::Many(exprs(vec![s(2, 2), s(1, 4)]))))
            )
        }

        #[test]
        fn many_steps() {
            assert_eq!(
                months_expr("1/2,FEB/3,3/4"),
                Ok(("", Expr::Many(exprs(vec![s(1, 2), s(2, 3), s(3, 4)]))))
            )
        }

        #[test]
        fn range_step() {
            assert_eq!(
                months_expr("1-DEC/4"),
                Ok(("", Expr::Many(exprs(vec![rs(1, 12, 4)]))))
            )
        }

        #[test]
        fn many_range_step() {
            assert_eq!(
                months_expr("1-JUN/4,JUN-12/3"),
                Ok(("", Expr::Many(exprs(vec![rs(1, 6, 4), rs(6, 12, 3)]))))
            )
        }

        #[test]
        fn values_ranges_steps_and_ranges() {
            assert_eq!(
                months_expr("1,JAN-6/3,JUN-12,DEC/3"),
                Ok((
                    "",
                    Expr::Many(exprs(vec![o(1), rs(1, 6, 3), r(6, 12), s(12, 3)]))
                ))
            )
        }

        #[test]
        fn limits() {
            assert!(matches!(months_expr("0"), Err(_)));
            assert!(matches!(months_expr("13"), Err(_)));
            assert!(matches!(months_expr("0-12"), Err(_)));
            assert!(matches!(months_expr("1-13"), Err(_)));
            // a step greater than the max value is not allowed (since it doesn't make sense)
            assert!(matches!(months_expr("1/13"), Err(_)));
            assert!(matches!(months_expr("1-13/2"), Err(_)));
            assert!(matches!(months_expr("0/12"), Err(_)));
            assert!(matches!(months_expr("0-12/2"), Err(_)));
            // a step of 0 is not allowed (since it doesn't make sense)
            assert!(matches!(months_expr("1/0"), Err(_)));
            assert!(matches!(months_expr("1-12/0"), Err(_)));
        }
    }

    mod days_of_month {
        use super::*;

        #[test]
        fn all() {
            assert_eq!(dom_expr("*"), Ok(("", DayOfMonthExpr::All)))
        }

        #[test]
        fn only_match_first_star() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(dom_expr("*,*"), Ok((",*", DayOfMonthExpr::All)))
        }

        #[test]
        fn last() {
            assert_eq!(dom_expr("L"), Ok(("", DayOfMonthExpr::Last(Last::Day))))
        }

        #[test]
        fn last_weekday() {
            assert_eq!(
                dom_expr("LW"),
                Ok(("", DayOfMonthExpr::Last(Last::Weekday)))
            )
        }

        #[test]
        fn last_offset() {
            assert_eq!(
                dom_expr("L-3"),
                Ok(("", DayOfMonthExpr::Last(Last::Offset(e(3)))))
            )
        }

        // a zero offset makes no sense, should just be L
        // a 32 offset will never execute
        #[test]
        fn last_offset_limit() {
            assert!(matches!(dom_expr("L-0"), Err(_)));
            assert!(matches!(dom_expr("L-31"), Err(_)));
            assert!(matches!(dom_expr("L-0W"), Err(_)));
            assert!(matches!(dom_expr("L-31W"), Err(_)));
        }

        #[test]
        fn last_offset_weekday() {
            assert_eq!(
                dom_expr("L-3W"),
                Ok(("", DayOfMonthExpr::Last(Last::OffsetWeekday(e(3)))))
            )
        }

        // last is not allowed with other expressions
        #[test]
        fn last_with_other_exprs() {
            assert!(matches!(dom_expr("3,L"), Err(_)))
        }

        #[test]
        fn closest_weekday() {
            assert_eq!(
                dom_expr("1W"),
                Ok(("", DayOfMonthExpr::ClosestWeekday(e(1))))
            )
        }

        #[test]
        fn closest_weekday_with_other_exprs() {
            // make sure we only match the 1W.
            // it'll fail on the next parser
            assert_eq!(
                dom_expr("1W,3"),
                Ok((",3", DayOfMonthExpr::ClosestWeekday(e(1))))
            )
        }

        #[test]
        fn star_step() {
            assert_eq!(
                dom_expr("*/3"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![s(1, 3)]))))
            )
        }

        #[test]
        fn multi_star_step() {
            assert_eq!(
                dom_expr("*/3,*/4"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![s(1, 3), s(1, 4)]))))
            )
        }

        #[test]
        fn star_range_doesnt_make_sense() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(dom_expr("*-6/3,*/4"), Ok(("-6/3,*/4", DayOfMonthExpr::All)))
        }

        #[test]
        fn one_value() {
            assert_eq!(
                dom_expr("1"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![o(1)]))))
            )
        }

        #[test]
        fn many_one_value() {
            assert_eq!(
                dom_expr("1,4,7,10,13,16,19,22,25,28,31"),
                Ok((
                    "",
                    DayOfMonthExpr::Many(exprs(vec![
                        o(1),
                        o(4),
                        o(7),
                        o(10),
                        o(13),
                        o(16),
                        o(19),
                        o(22),
                        o(25),
                        o(28),
                        o(31),
                    ]))
                ))
            )
        }

        #[test]
        fn one_range() {
            assert_eq!(
                dom_expr("1-15"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![r(1, 15)]))))
            )
        }

        #[test]
        fn overflow_range() {
            assert_eq!(
                dom_expr("30-1"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![r(30, 1)]))))
            )
        }

        #[test]
        fn many_range() {
            assert_eq!(
                dom_expr("1-4,5-8,9-12,13-15"),
                Ok((
                    "",
                    DayOfMonthExpr::Many(exprs(vec![r(1, 4), r(5, 8), r(9, 12), r(13, 15)]))
                ))
            )
        }

        #[test]
        fn step() {
            assert_eq!(
                dom_expr("1/3"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![s(1, 3)]))))
            )
        }

        #[test]
        fn step_with_star_step() {
            assert_eq!(
                dom_expr("2/2,*/4"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![s(2, 2), s(1, 4)]))))
            )
        }

        #[test]
        fn many_steps() {
            assert_eq!(
                dom_expr("1/2,2/3,3/4"),
                Ok((
                    "",
                    DayOfMonthExpr::Many(exprs(vec![s(1, 2), s(2, 3), s(3, 4)]))
                ))
            )
        }

        #[test]
        fn range_step() {
            assert_eq!(
                dom_expr("1-15/4"),
                Ok(("", DayOfMonthExpr::Many(exprs(vec![rs(1, 15, 4)]))))
            )
        }

        #[test]
        fn many_range_step() {
            assert_eq!(
                dom_expr("1-15/3,16-31/4"),
                Ok((
                    "",
                    DayOfMonthExpr::Many(exprs(vec![rs(1, 15, 3), rs(16, 31, 4)]))
                ))
            )
        }

        #[test]
        fn values_ranges_steps_and_ranges() {
            assert_eq!(
                dom_expr("1,1-10/3,10-20,20/3"),
                Ok((
                    "",
                    DayOfMonthExpr::Many(exprs(vec![o(1), rs(1, 10, 3), r(10, 20), s(20, 3)]))
                ))
            )
        }

        #[test]
        fn limits() {
            assert!(matches!(dom_expr("32"), Err(_)));
            assert!(matches!(dom_expr("0-31"), Err(_)));
            assert!(matches!(dom_expr("1-32"), Err(_)));
            // a step greater than the max value is not allowed (since it doesn't make sense)
            assert!(matches!(dom_expr("1/32"), Err(_)));
            assert!(matches!(dom_expr("0/31"), Err(_)));
            assert!(matches!(dom_expr("1-31/32"), Err(_)));
            assert!(matches!(dom_expr("0-31/32"), Err(_)));
            assert!(matches!(dom_expr("0-32/31"), Err(_)));
            // a step of 0 is not allowed (since it doesn't make sense)
            assert!(matches!(dom_expr("0/0"), Err(_)));
            assert!(matches!(dom_expr("0-23/0"), Err(_)));
        }
    }

    mod days_of_week {
        use super::*;

        #[test]
        fn all() {
            assert_eq!(dow_expr("*"), Ok(("", DayOfWeekExpr::All)))
        }

        #[test]
        fn only_match_first_star() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(dow_expr("*,*"), Ok((",*", DayOfWeekExpr::All)))
        }

        #[test]
        fn last() {
            assert_eq!(
                dow_expr("L"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(7)]))))
            )
        }

        #[test]
        fn last_day() {
            assert_eq!(dow_expr("3L"), Ok(("", DayOfWeekExpr::Last(e(3)))))
        }

        // last is not allowed with other expressions
        #[test]
        fn last_with_other_exprs() {
            assert!(matches!(dow_expr("3,L"), Err(_)))
        }

        #[test]
        fn nth() {
            assert_eq!(dow_expr("MON#1"), Ok(("", DayOfWeekExpr::Nth(e(2), e(1)))));
            assert_eq!(dow_expr("5#4"), Ok(("", DayOfWeekExpr::Nth(e(5), e(4)))));
        }

        #[test]
        fn star_step() {
            assert_eq!(
                dow_expr("*/3"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![s(1, 3)]))))
            )
        }

        #[test]
        fn multi_star_step() {
            assert_eq!(
                dow_expr("*/3,*/4"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![s(1, 3), s(1, 4)]))))
            )
        }

        #[test]
        fn star_range_doesnt_make_sense() {
            // make sure we only match the first star.
            // it'll fail on the next parser
            assert_eq!(dow_expr("*-6/3,*/4"), Ok(("-6/3,*/4", DayOfWeekExpr::All)))
        }

        #[test]
        fn one_value() {
            assert_eq!(
                dow_expr("1"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(1)]))))
            )
        }

        #[test]
        fn word_values() {
            // caps
            assert_eq!(
                dow_expr("SUN"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(1)]))))
            );
            assert_eq!(
                dow_expr("MON"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(2)]))))
            );
            assert_eq!(
                dow_expr("TUE"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(3)]))))
            );

            // lower
            assert_eq!(
                dow_expr("WED"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(4)]))))
            );
            assert_eq!(
                dow_expr("THU"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(5)]))))
            );
            assert_eq!(
                dow_expr("FRI"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(6)]))))
            );

            // mixed
            assert_eq!(
                dow_expr("SaT"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(7)]))))
            );
        }

        #[test]
        fn many_one_value() {
            assert_eq!(
                dow_expr("2,WED,FRI,7"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![o(2), o(4), o(6), o(7)]))))
            )
        }

        #[test]
        fn one_range() {
            assert_eq!(
                dow_expr("MON-5"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![r(2, 5)]))))
            )
        }

        #[test]
        fn overflow_range() {
            assert_eq!(
                dow_expr("7-1"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![r(7, 1)]))))
            )
        }

        #[test]
        fn many_range() {
            assert_eq!(
                dow_expr("1-3,4-4,5-7"),
                Ok((
                    "",
                    DayOfWeekExpr::Many(exprs(vec![r(1, 3), r(4, 4), r(5, 7)]))
                ))
            )
        }

        #[test]
        fn step() {
            assert_eq!(
                dow_expr("2/2"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![s(2, 2)]))))
            )
        }

        #[test]
        fn step_with_star_step() {
            assert_eq!(
                dow_expr("2/2,*/4"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![s(2, 2), s(1, 4)]))))
            )
        }

        #[test]
        fn many_steps() {
            assert_eq!(
                dow_expr("1/2,2/3,3/4"),
                Ok((
                    "",
                    DayOfWeekExpr::Many(exprs(vec![s(1, 2), s(2, 3), s(3, 4)]))
                ))
            )
        }

        #[test]
        fn range_step() {
            assert_eq!(
                dow_expr("2-5/2"),
                Ok(("", DayOfWeekExpr::Many(exprs(vec![rs(2, 5, 2)]))))
            )
        }

        #[test]
        fn many_range_step() {
            assert_eq!(
                dow_expr("1-4/2,5-7/2"),
                Ok((
                    "",
                    DayOfWeekExpr::Many(exprs(vec![rs(1, 4, 2), rs(5, 7, 2)]))
                ))
            )
        }

        #[test]
        fn values_ranges_steps_and_ranges() {
            assert_eq!(
                dow_expr("1,2-FRI/2,6-7,3/3"),
                Ok((
                    "",
                    DayOfWeekExpr::Many(exprs(vec![o(1), rs(2, 6, 2), r(6, 7), s(3, 3)]))
                ))
            )
        }

        #[test]
        fn limits() {
            assert!(matches!(dow_expr("8"), Err(_)));
            assert!(matches!(dow_expr("0"), Err(_)));
            assert!(matches!(dow_expr("0-7"), Err(_)));
            assert!(matches!(dow_expr("1-8"), Err(_)));
            // a step greater than the max value is not allowed (since it doesn't make sense)
            assert!(matches!(dow_expr("1/8"), Err(_)));
            assert!(matches!(dow_expr("0/7"), Err(_)));
            assert!(matches!(dow_expr("1-7/8"), Err(_)));
            assert!(matches!(dow_expr("0-7/7"), Err(_)));
            assert!(matches!(dow_expr("0-8/7"), Err(_)));
            // a step of 0 is not allowed (since it doesn't make sense)
            assert!(matches!(dow_expr("1/0"), Err(_)));
            assert!(matches!(dow_expr("1-5/0"), Err(_)));
            // 0th day doesn't make sense
            assert!(matches!(dow_expr("SUN#0"), Err(_)));
            // 6th day of the month will never happen
            assert!(matches!(dow_expr("MON#6"), Err(_)));
        }
    }
}
