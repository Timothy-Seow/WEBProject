use serde::Deserialize;

// Optional date range used by report and list pages.
#[derive(Debug, Deserialize)]
pub(crate) struct DateRangeFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MonthFilter {
    pub month: Option<String>,
}
