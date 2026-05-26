//! Date literals, ranges, and the strict-before comparison.
//!
//! Spec section 3.5 defines three granularities (`YYYY`, `YYYY-MM`,
//! `YYYY-MM-DD`) and an optional `~` (circa) prefix that adds ±5 years of
//! tolerance. Comparisons treat partial / circa dates as closed intervals
//! and only fire when one interval is strictly before the other.

use crate::span::ByteSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateLit {
    pub span: ByteSpan,
    pub circa: bool,
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

impl DateLit {
    /// Canonical Kul source rendering: `[~]YYYY[-MM[-DD]]`.
    ///
    /// The single source of truth for how a date appears in `.kul` source
    /// after formatting and in tooling output (LSP hover, diagnostics).
    #[must_use]
    pub fn format_canonical(&self) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(11);
        if self.circa {
            s.push('~');
        }
        write!(s, "{:04}", self.year).expect("write year");
        if let Some(m) = self.month {
            write!(s, "-{:02}", m).expect("write month");
        }
        if let Some(d) = self.day {
            write!(s, "-{:02}", d).expect("write day");
        }
        s
    }

    /// Year-only short form: `[~]YYYY`. Used by the LSP (completion details,
    /// document-symbol details) to show a compact year without the
    /// month/day noise.
    #[must_use]
    pub fn format_year(&self) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(5);
        if self.circa {
            s.push('~');
        }
        write!(s, "{:04}", self.year).expect("write year");
        s
    }

    #[must_use]
    pub fn lower_bound(&self) -> CalendarDay {
        let mut start = match (self.month, self.day) {
            (Some(m), Some(d)) => CalendarDay::new(self.year as i32, m as i32, d as i32),
            (Some(m), None) => CalendarDay::new(self.year as i32, m as i32, 1),
            (None, _) => CalendarDay::new(self.year as i32, 1, 1),
        };
        if self.circa {
            start = start.add_years(-5);
        }
        start
    }

    #[must_use]
    pub fn upper_bound(&self) -> CalendarDay {
        let mut end = match (self.month, self.day) {
            (Some(m), Some(d)) => CalendarDay::new(self.year as i32, m as i32, d as i32),
            (Some(m), None) => CalendarDay::new(
                self.year as i32,
                m as i32,
                days_in_month(self.year, m) as i32,
            ),
            (None, _) => CalendarDay::new(self.year as i32, 12, 31),
        };
        if self.circa {
            end = end.add_years(5);
        }
        end
    }
}

/// Return true iff every interpretation of `a` is strictly before every
/// interpretation of `b`.
#[must_use]
pub fn before_strict(a: &DateLit, b: &DateLit) -> bool {
    a.upper_bound() < b.lower_bound()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalendarDay {
    pub year: i32,
    pub month: i32,
    pub day: i32,
}

impl CalendarDay {
    pub const fn new(year: i32, month: i32, day: i32) -> Self {
        Self { year, month, day }
    }

    fn add_years(self, n: i32) -> Self {
        Self {
            year: self.year + n,
            month: self.month,
            day: self.day,
        }
    }
}

/// Parse a date literal as written (e.g. `~1925-03-15`, `1950-04`, `2010`).
/// Returns `Err` if the form is malformed or if the components are invalid
/// (month not in 1..=12, day not valid for the month).
#[must_use = "parsing a date is pointless if the result is discarded"]
pub fn parse_date(raw: &str, span: ByteSpan) -> Result<DateLit, DateParseError> {
    let (circa, body) = if let Some(rest) = raw.strip_prefix('~') {
        (true, rest)
    } else {
        (false, raw)
    };

    let parts: Vec<&str> = body.split('-').collect();
    let (year_str, month_str, day_str) = match parts.as_slice() {
        [y] => (*y, None, None),
        [y, m] => (*y, Some(*m), None),
        [y, m, d] => (*y, Some(*m), Some(*d)),
        _ => return Err(DateParseError::Malformed(format!("`{raw}` is not a date"))),
    };

    if year_str.len() != 4 || !year_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(DateParseError::Malformed(format!(
            "year `{year_str}` must be exactly 4 digits"
        )));
    }
    let year: u16 = year_str
        .parse()
        .map_err(|_| DateParseError::Malformed(format!("invalid year `{year_str}`")))?;

    let month = if let Some(m_str) = month_str {
        if m_str.len() != 2 || !m_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(DateParseError::Malformed(format!(
                "month `{m_str}` must be exactly 2 digits"
            )));
        }
        let m: u8 = m_str
            .parse()
            .map_err(|_| DateParseError::Malformed(format!("invalid month `{m_str}`")))?;
        if !(1..=12).contains(&m) {
            return Err(DateParseError::OutOfRange(format!(
                "month `{m_str}` is out of range (must be 01..12)"
            )));
        }
        Some(m)
    } else {
        None
    };

    let day = if let Some(d_str) = day_str {
        if d_str.len() != 2 || !d_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(DateParseError::Malformed(format!(
                "day `{d_str}` must be exactly 2 digits"
            )));
        }
        let d: u8 = d_str
            .parse()
            .map_err(|_| DateParseError::Malformed(format!("invalid day `{d_str}`")))?;
        let m = month.expect("day requires month per grammar");
        let dim = days_in_month(year, m);
        if !(1..=dim).contains(&d) {
            return Err(DateParseError::OutOfRange(format!(
                "day `{d_str}` is out of range for {year:04}-{m:02} (1..{dim})"
            )));
        }
        Some(d)
    } else {
        None
    };

    Ok(DateLit {
        span,
        circa,
        year,
        month,
        day,
    })
}

#[derive(Debug, Clone)]
pub enum DateParseError {
    Malformed(String),
    OutOfRange(String),
}

impl DateParseError {
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            DateParseError::Malformed(m) | DateParseError::OutOfRange(m) => m,
        }
    }
}

#[must_use]
pub fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[must_use]
pub fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> DateLit {
        parse_date(s, ByteSpan::new(0, s.len())).expect("valid")
    }

    #[test]
    fn parses_full_date() {
        let lit = d("1975-09-03");
        assert_eq!(lit.year, 1975);
        assert_eq!(lit.month, Some(9));
        assert_eq!(lit.day, Some(3));
        assert!(!lit.circa);
    }

    #[test]
    fn parses_year_only() {
        let lit = d("1925");
        assert_eq!(lit.year, 1925);
        assert_eq!(lit.month, None);
        assert_eq!(lit.day, None);
    }

    #[test]
    fn parses_circa() {
        let lit = d("~1925");
        assert!(lit.circa);
        assert_eq!(lit.year, 1925);
    }

    #[test]
    fn rejects_feb_30() {
        let res = parse_date("1950-02-30", ByteSpan::new(0, 10));
        assert!(matches!(res, Err(DateParseError::OutOfRange(_))));
    }

    #[test]
    fn rejects_feb_29_non_leap() {
        let res = parse_date("1925-02-29", ByteSpan::new(0, 10));
        assert!(matches!(res, Err(DateParseError::OutOfRange(_))));
    }

    #[test]
    fn accepts_feb_29_leap() {
        let lit = d("2000-02-29");
        assert_eq!(lit.day, Some(29));
    }

    #[test]
    fn rejects_month_zero() {
        assert!(matches!(
            parse_date("1925-00", ByteSpan::new(0, 7)),
            Err(DateParseError::OutOfRange(_))
        ));
    }

    #[test]
    fn rejects_month_thirteen() {
        assert!(matches!(
            parse_date("1925-13", ByteSpan::new(0, 7)),
            Err(DateParseError::OutOfRange(_))
        ));
    }

    #[test]
    fn before_strict_basic() {
        assert!(before_strict(&d("1900"), &d("1901")));
        assert!(!before_strict(&d("1900"), &d("1900")));
    }

    #[test]
    fn before_strict_partial_overlap() {
        // `1900` covers all of 1900; `1900-06` is mid-1900 — overlap.
        assert!(!before_strict(&d("1900"), &d("1900-06")));
        assert!(!before_strict(&d("1900-06"), &d("1900")));
    }

    #[test]
    fn before_strict_circa_overlap() {
        // ~1900 covers 1895..1905; 1903 is inside — not strictly before.
        assert!(!before_strict(&d("~1900"), &d("1903")));
        // ~1900 ends at 1905-12-31; 1906 is strictly after.
        assert!(before_strict(&d("~1900"), &d("1906")));
    }

    #[test]
    fn before_strict_full_dates() {
        assert!(before_strict(&d("1950-01-01"), &d("1950-01-02")));
        assert!(!before_strict(&d("1950-01-02"), &d("1950-01-02")));
    }

    #[test]
    fn format_canonical_round_trips_each_precision() {
        assert_eq!(d("1975-09-03").format_canonical(), "1975-09-03");
        assert_eq!(d("1975-09").format_canonical(), "1975-09");
        assert_eq!(d("1975").format_canonical(), "1975");
        assert_eq!(d("~1925").format_canonical(), "~1925");
        assert_eq!(d("~1925-12-31").format_canonical(), "~1925-12-31");
    }

    #[test]
    fn format_year_drops_month_and_day() {
        assert_eq!(d("1975-09-03").format_year(), "1975");
        assert_eq!(d("~1925-12").format_year(), "~1925");
        assert_eq!(d("1925").format_year(), "1925");
    }
}
