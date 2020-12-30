use crate::describe::{display, Language};
use crate::parse::*;
use chrono::NaiveTime;
use core::fmt::{self, Display, Formatter};

fn postfixed<T: Into<usize>>(x: T) -> impl Display {
    let x: usize = x.into();
    display(move |f| match x % 100 {
        1 => write!(f, "{}st", x),
        2 => write!(f, "{}nd", x),
        3 => write!(f, "{}rd", x),
        20..=99 => match x % 10 {
            1 => write!(f, "{}st", x),
            2 => write!(f, "{}nd", x),
            3 => write!(f, "{}rd", x),
            _ => write!(f, "{}th", x),
        },
        _ => write!(f, "{}th", x),
    })
}

fn weekday<T: Into<chrono::Weekday>>(x: T) -> impl Display {
    use chrono::Weekday::*;
    let x: chrono::Weekday = x.into();
    display(move |f| match x {
        Mon => write!(f, "Monday"),
        Tue => write!(f, "Tuesday"),
        Wed => write!(f, "Wednesday"),
        Thu => write!(f, "Thursday"),
        Fri => write!(f, "Friday"),
        Sat => write!(f, "Saturday"),
        Sun => write!(f, "Sunday"),
    })
}

/// Specifies whether to display times with a 12 hour or 24 hour clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HourFormat {
    /// Format using a 12 hour clock (i.e. 6:30 PM)
    Hour12,
    /// Format using a 24 hour clock (i.e. 18:30)
    Hour24,
}

impl Default for HourFormat {
    fn default() -> Self {
        HourFormat::Hour12
    }
}

/// English language formatting
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct English {
    /// Configures how hours are formatted in descriptions
    pub hour: HourFormat,
}

impl English {
    /// Creates a new instance of the english configuration with its default values
    pub const fn new() -> Self {
        Self {
            hour: HourFormat::Hour12,
        }
    }
}

impl Default for English {
    fn default() -> Self {
        Self::new()
    }
}

impl English {
    fn minute(&self, h: OrsExpr<Minute>) -> impl Display {
        display(move |f| match h {
            OrsExpr::One(minute) => write!(f, "{}", u8::from(minute)),
            OrsExpr::Range(start, end) => {
                write!(f, "{} through {}", u8::from(start), u8::from(end))
            }
            OrsExpr::Step { start, end, step } => write!(
                f,
                "every {} minute from {} through {}",
                postfixed(u8::from(step)),
                u8::from(start),
                u8::from(end)
            ),
        })
    }
    fn hour<'a>(&'a self, h: OrsExpr<Hour>) -> impl Display + 'a {
        display(move |f| match h {
            OrsExpr::One(hour) => write!(
                f,
                "between {} and {}",
                self.time(hour, 0),
                self.time(hour, 59)
            ),
            OrsExpr::Range(start, end) => write!(
                f,
                "between {} and {}",
                self.time(start, 0),
                self.time(end, 59)
            ),
            OrsExpr::Step { start, end, step } => write!(
                f,
                "every {} hour between {} and {}",
                postfixed(u8::from(step)),
                self.time(start, 0),
                self.time(end, 59)
            ),
        })
    }
    fn month(&self, h: OrsExpr<Month>) -> impl Display {
        display(move |f| match h {
            OrsExpr::One(month) => write!(f, "{}", chrono::Month::from(month).name()),
            OrsExpr::Range(start, end) => write!(
                f,
                "{} to {}",
                chrono::Month::from(start).name(),
                chrono::Month::from(end).name()
            ),
            OrsExpr::Step { start, end, step } => write!(
                f,
                "every {} month from {} to {}",
                postfixed(u8::from(step)),
                chrono::Month::from(start).name(),
                chrono::Month::from(end).name()
            ),
        })
    }
    fn day_of_week(&self, h: OrsExpr<DayOfWeek>) -> impl Display {
        display(move |f| match h {
            OrsExpr::One(dow) => write!(f, "{}", weekday(dow)),
            OrsExpr::Range(start, end) => write!(f, "{} through {}", weekday(start), weekday(end)),
            OrsExpr::Step { start, end, step } => write!(
                f,
                "every {} weekday {} through {}",
                postfixed(u8::from(step)),
                weekday(start),
                weekday(end)
            ),
        })
    }
    fn day_of_month(&self, h: OrsExpr<DayOfMonth>) -> impl Display {
        display(move |f| match h {
            OrsExpr::One(dom) => write!(f, "{}", postfixed(u8::from(dom) + 1)),
            OrsExpr::Range(start, end) => write!(
                f,
                "{} to {}",
                postfixed(u8::from(start) + 1),
                postfixed(u8::from(end) + 1)
            ),
            OrsExpr::Step { start, end, step } => write!(
                f,
                "every {} day from the {} to the {}",
                postfixed(u8::from(step)),
                postfixed(u8::from(start) + 1),
                postfixed(u8::from(end) + 1)
            ),
        })
    }
    fn time<H: Into<u8>, M: Into<u8>>(&self, hour: H, minute: M) -> impl Display {
        let time = NaiveTime::from_hms(hour.into() as u32, minute.into() as u32, 0);
        let fmt = match self.hour {
            HourFormat::Hour12 => "%-I:%M %p",
            HourFormat::Hour24 => "%H:%M",
        };
        time.format(fmt)
    }
}
impl Language for English {
    fn fmt_expr(&self, expr: &CronExpr, f: &mut Formatter) -> fmt::Result {
        match (&expr.minutes, &expr.hours) {
            (Expr::All, Expr::All) => write!(f, "Every minute")?,
            (Expr::All, Expr::Many(Exprs { first, tail })) => {
                let first = first.normalize();
                write!(f, "Every minute ")?;
                match tail.as_slice() {
                    [] => write!(f, "{}", self.hour(first))?,
                    [second] => write!(
                        f,
                        "{} and {}",
                        self.hour(first),
                        self.hour(second.normalize())
                    )?,
                    [middle @ .., last] => {
                        write!(f, "{}, ", self.hour(first))?;
                        for expr in middle {
                            write!(f, "{}, ", self.hour(expr.normalize()))?;
                        }
                        write!(f, "and {}", self.hour(last.normalize()))?;
                    }
                }
            }
            (Expr::Many(Exprs { first, tail }), Expr::All) => {
                let first = first.normalize();
                match tail.as_slice() {
                    [] => match first {
                        OrsExpr::One(value) => match u8::from(value) {
                            0 => write!(f, "Every hour"),
                            1 => write!(f, "At 1 minute past the hour"),
                            v => write!(f, "At {} minutes past the hour", v),
                        }?,
                        OrsExpr::Range(start, end) => write!(
                            f,
                            "Minutes {} through {} past the hour",
                            u8::from(start),
                            u8::from(end)
                        )?,
                        OrsExpr::Step { start, end, step } => write!(
                            f,
                            "Every {} minute starting from minute {} to minute {} past the hour",
                            postfixed(u8::from(step)),
                            u8::from(start),
                            u8::from(end),
                        )?,
                    },
                    [second] => write!(
                        f,
                        "At {} and {} minutes past the hour",
                        self.minute(first),
                        self.minute(second.normalize())
                    )?,
                    [middle @ .., last] => {
                        write!(f, "At {}, ", self.minute(first))?;
                        for expr in middle {
                            write!(f, "{}, ", self.minute(expr.normalize()))?;
                        }
                        write!(
                            f,
                            "and {} minutes past the hour",
                            self.minute(last.normalize())
                        )?;
                    }
                }
            }
            (
                Expr::Many(Exprs {
                    first: first_minute,
                    tail: tail_minutes,
                }),
                Expr::Many(Exprs {
                    first: first_hour,
                    tail: tail_hours,
                }),
            ) => {
                let first_minute = first_minute.normalize();
                let tail_minutes = tail_minutes.as_slice();
                let first_hour = first_hour.normalize();
                let tail_hours = tail_hours.as_slice();
                if let (OrsExpr::One(minute), [], OrsExpr::One(hour), []) =
                    (first_minute, tail_minutes, first_hour, tail_hours)
                {
                    write!(f, "At {}", self.time(hour, minute))?;
                } else {
                    match tail_minutes {
                        [] => write!(
                            f,
                            "At {} minutes past the hour, ",
                            self.minute(first_minute)
                        )?,
                        [second] => write!(
                            f,
                            "At {} and {} minutes past the hour, ",
                            self.minute(first_minute),
                            self.minute(second.normalize())
                        )?,
                        [middle @ .., last] => {
                            write!(f, "At {}, ", self.minute(first_minute))?;
                            for expr in middle {
                                write!(f, "{}, ", self.minute(expr.normalize()))?;
                            }
                            write!(f, "and {}, ", self.minute(last.normalize()))?;
                        }
                    }

                    match tail_hours {
                        [] => write!(f, "{}", self.hour(first_hour))?,
                        [second] => write!(
                            f,
                            "{} and {}",
                            self.hour(first_hour),
                            self.hour(second.normalize())
                        )?,
                        [middle @ .., last] => {
                            write!(f, "{}, ", self.hour(first_hour))?;
                            for expr in middle {
                                write!(f, "{}, ", self.hour(expr.normalize()))?;
                            }
                            write!(f, "and {}", self.hour(last.normalize()))?;
                        }
                    }
                }
            }
        }

        match &expr.doms {
            DayOfMonthExpr::All => {}
            &DayOfMonthExpr::ClosestWeekday(day) => write!(
                f,
                " on the closest weekday to the {}",
                postfixed(u8::from(day) + 1)
            )?,
            DayOfMonthExpr::Last(Last::Day) => write!(f, " on the last day")?,
            DayOfMonthExpr::Last(Last::Weekday) => write!(f, " on the last weekday")?,
            &DayOfMonthExpr::Last(Last::Offset(offset)) => {
                write!(f, " on the {} to last day", postfixed(u8::from(offset) + 1))?
            }
            &DayOfMonthExpr::Last(Last::OffsetWeekday(offset)) => write!(
                f,
                " on the closest weekday to the {} to last day",
                postfixed(u8::from(offset) + 1)
            )?,
            DayOfMonthExpr::Many(Exprs { first, tail }) => {
                let first = first.normalize();
                match tail.as_slice() {
                    [] => write!(f, " on the {}", self.day_of_month(first))?,
                    [second] => write!(
                        f,
                        " on the {} and {}",
                        self.day_of_month(first),
                        self.day_of_month(second.normalize())
                    )?,
                    [middle @ .., last] => {
                        write!(f, " on the {}, ", self.day_of_month(first))?;
                        for expr in middle {
                            write!(f, "{}, ", self.day_of_month(expr.normalize()))?;
                        }
                        write!(f, "and {}", self.day_of_month(last.normalize()))?;
                    }
                }
            }
        }

        match (&expr.doms, &expr.dows) {
            (DayOfMonthExpr::All, _) | (_, DayOfWeekExpr::All) => {}
            _ => write!(f, " and")?,
        }

        match &expr.dows {
            DayOfWeekExpr::All => {}
            &DayOfWeekExpr::Last(day) => write!(f, " on the last {}", weekday(day))?,
            &DayOfWeekExpr::Nth(day, nth) => {
                write!(f, " on the {} {}", postfixed(u8::from(nth)), weekday(day))?
            }
            DayOfWeekExpr::Many(Exprs { first, tail }) => {
                let first = first.normalize();
                match tail.as_slice() {
                    [] => write!(f, " on {}", self.day_of_week(first))?,
                    [second] => write!(
                        f,
                        " on {} and {}",
                        self.day_of_week(first),
                        self.day_of_week(second.normalize())
                    )?,
                    [middle @ .., last] => {
                        write!(f, " on {}, ", self.day_of_week(first))?;
                        for expr in middle {
                            write!(f, "{}, ", self.day_of_week(expr.normalize()))?;
                        }
                        write!(f, "and {}", self.day_of_week(last.normalize()))?;
                    }
                }
            }
        }

        let Exprs { first, tail } = match (&expr.doms, &expr.months, &expr.dows) {
            (DayOfMonthExpr::All, Expr::All, DayOfWeekExpr::All)
            | (DayOfMonthExpr::All, Expr::All, DayOfWeekExpr::Many(_)) => return Ok(()),
            (_, Expr::All, _) => {
                write!(f, " of every month")?;
                return Ok(());
            }
            (DayOfMonthExpr::All, Expr::Many(exprs), DayOfWeekExpr::All) => {
                write!(f, " every day in ")?;
                exprs
            }
            (_, Expr::Many(exprs), _) => {
                write!(f, " of ")?;
                exprs
            }
        };

        let first = first.normalize();
        match tail.as_slice() {
            [] => write!(f, "{}", self.month(first))?,
            [second] => write!(
                f,
                "{} and {}",
                self.month(first),
                self.month(second.normalize())
            )?,
            [middle @ .., last] => {
                write!(f, "{}, ", self.month(first))?;
                for expr in middle {
                    write!(f, "{}, ", self.month(expr.normalize()))?;
                }
                write!(f, "and {}", self.month(last.normalize()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    const CFG_24_HOURS: English = English {
        hour: HourFormat::Hour24,
        ..English::new()
    };

    #[track_caller]
    fn assert_cfg(cfg: English, cron: &str, expected: &str) {
        let expr: CronExpr = cron.parse().expect("Valid cron expression");
        let description = expr.describe(cfg).to_string();

        assert_eq!(description, expected);
    }

    #[track_caller]
    fn assert(cron: &str, expected: &str) {
        let expr: CronExpr = cron.parse().expect("Valid cron expression");
        let description = expr.describe(English::new()).to_string();

        assert_eq!(description, expected);
    }

    #[test]
    fn time() {
        assert("* * * * *", "Every minute");
        assert("0 * * * *", "Every hour");
        assert("0 0 * * *", "At 12:00 AM");
        assert_cfg(CFG_24_HOURS, "0 0 * * *", "At 00:00");
        assert("0,1 * * * *", "At 0 and 1 minutes past the hour");
        assert(
            "0,1-5,10-30/2 * * * *",
            "At 0, 1 through 5, and every 2nd minute from 10 through 30 minutes past the hour",
        );
        assert(
            "0 2,3 * * *",
            "At 0 minutes past the hour, between 2:00 AM and 2:59 AM and between 3:00 AM and 3:59 AM",
        );
        assert(
            "0 2,5-10,*/2 * * *",
            "At 0 minutes past the hour, between 2:00 AM and 2:59 AM, between 5:00 AM and 10:59 AM, and every 2nd hour between 12:00 AM and 11:59 PM",
        );
    }

    #[test]
    fn day_of_month() {
        assert("* * L * *", "Every minute on the last day of every month");
        assert(
            "* * LW * *",
            "Every minute on the last weekday of every month",
        );
        assert(
            "* * L-1 * *",
            "Every minute on the 2nd to last day of every month",
        );
        assert(
            "* * L-1W * *",
            "Every minute on the closest weekday to the 2nd to last day of every month",
        );
        assert(
            "* * 15W * *",
            "Every minute on the closest weekday to the 15th of every month",
        );
        assert("* * 15 * *", "Every minute on the 15th of every month");
        assert(
            "* * 1,15 * *",
            "Every minute on the 1st and 15th of every month",
        );
        assert(
            "* * 1,10-20,20/2 * *",
            "Every minute on the 1st, 10th to 20th, and every 2nd day from the 20th to the 31st of every month"
        );
    }

    #[test]
    fn months() {
        assert("* * * FEB *", "Every minute every day in February");
        assert(
            "* * * JAN,FEB *",
            "Every minute every day in January and February",
        );
        assert(
            "* * * JAN,JUN-AUG,*/2 *",
            "Every minute every day in January, June to August, and every 2nd month from January to December"
        );
    }

    #[test]
    fn complex() {
        // test some complex expressions with all fields filled
        assert(
            "0 0 LW */2 FRIL",
            "At 12:00 AM on the last weekday and on the last Friday of every 2nd month from January to December"
        );
        assert(
            "0 0,12 L FEB FRI",
            "At 0 minutes past the hour, between 12:00 AM and 12:59 AM and between 12:00 PM and 12:59 PM on the last day and on Friday of February"
        );
    }

    #[test]
    fn day_of_week() {
        assert(
            "* * * * MONL",
            "Every minute on the last Monday of every month",
        );
        assert(
            "* * * * MON#5",
            "Every minute on the 5th Monday of every month",
        );
        assert("* * * * MON", "Every minute on Monday");
        assert("* * * * SUN,SAT", "Every minute on Sunday and Saturday");
        assert("* * * * */3,SAT,MON-FRI", "Every minute on every 3rd weekday Sunday through Saturday, Saturday, and Monday through Friday");
    }
}
