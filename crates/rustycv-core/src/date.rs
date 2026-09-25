use serde::{Deserialize, Serialize};

/// A month-precision date.
///
/// Deliberately *not* a free-text string: templates decide how dates are
/// formatted, so switching template restyles "Jan 2026" into "01/2026"
/// without touching stored data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateSpec {
    pub year: i32,
    /// 1-12, or `None` for year-only precision ("2017 – 2022").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u32>,
}

impl DateSpec {
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month: Some(month.clamp(1, 12)),
        }
    }

    pub fn year_only(year: i32) -> Self {
        Self { year, month: None }
    }

    /// Normalise an out-of-range month to `None` rather than rendering garbage.
    pub fn normalized(self) -> Self {
        match self.month {
            Some(m) if (1..=12).contains(&m) => self,
            Some(_) => Self {
                year: self.year,
                month: None,
            },
            None => self,
        }
    }
}
