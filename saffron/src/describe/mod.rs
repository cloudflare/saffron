mod english;

pub use english::{English, HourFormat};

use crate::parse::CronExpr;
use core::fmt::{self, Display, Formatter};

/// A language formatting configuration
pub trait Language {
    /// Formats a cron expression into the specified formatter
    fn fmt_expr(&self, expr: &CronExpr, f: &mut Formatter) -> fmt::Result;
}

impl<'a, L: Language> Language for &'a L {
    fn fmt_expr(&self, expr: &CronExpr, f: &mut Formatter) -> fmt::Result {
        (*self).fmt_expr(expr, f)
    }
}

struct Displayer<F>(pub F);
impl<F> Display for Displayer<F>
where
    F: Fn(&mut Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0(f)
    }
}

fn display<F>(f: F) -> Displayer<F>
where
    F: Fn(&mut Formatter) -> fmt::Result,
{
    Displayer(f)
}
