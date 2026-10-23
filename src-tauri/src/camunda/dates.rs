use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};

use crate::models::{DashboardDateFilter, DashboardDatePreset};

pub const CAMUNDA_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f%z";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamundaDateRange {
    pub after: String,
    pub before: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineBucket {
    pub label: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

// Разбирает дату Camunda `yyyy-MM-dd'T'HH:mm:ss.SSSZ`, при неудаче пробует RFC 3339.
pub fn parse_camunda_date_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    DateTime::parse_from_str(value, CAMUNDA_DATE_FORMAT)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .ok()
}

pub fn format_camunda_date_time(value: &DateTime<Local>) -> String {
    value.format(CAMUNDA_DATE_FORMAT).to_string()
}

// Диапазон для дашборда: для «Сейчас» это последние 24 часа, иначе выбранные даты.
pub fn dashboard_range(filter: &DashboardDateFilter) -> Result<CamundaDateRange, String> {
    if !filter.is_live() {
        return camunda_date_range(filter);
    }
    let end = Local::now();
    let start = end - Duration::hours(24);
    Ok(CamundaDateRange { after: format_camunda_date_time(&start), before: format_camunda_date_time(&end), start, end })
}

// Границы периода в локальной таймзоне: after — за миллисекунду до начала первого дня, before — начало дня после последнего.
pub fn camunda_date_range(filter: &DashboardDateFilter) -> Result<CamundaDateRange, String> {
    let today = Local::now().date_naive();
    let (start_date, end_date) = match filter.preset {
        DashboardDatePreset::Live => return Err("Для текущей статистики диапазон дат не используется".to_string()),
        DashboardDatePreset::Today => (today, today),
        DashboardDatePreset::Yesterday => {
            let yesterday = today.pred_opt().unwrap_or(today);
            (yesterday, yesterday)
        }
        DashboardDatePreset::Custom => {
            let start = parse_date(&filter.from_date)?;
            let end = parse_date(&filter.to_date)?;
            if start > end {
                return Err("Начальная дата не может быть позже конечной.".to_string());
            }
            (start, end)
        }
    };
    let start = local_midnight(start_date);
    let end = local_midnight(end_date.succ_opt().ok_or_else(|| "Введите дату в формате ГГГГ-ММ-ДД.".to_string())?);
    Ok(CamundaDateRange {
        after: format_camunda_date_time(&(start - Duration::milliseconds(1))),
        before: format_camunda_date_time(&end),
        start,
        end,
    })
}

pub fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| "Введите дату в формате ГГГГ-ММ-ДД.".to_string())
}

pub fn timeline_title(filter: &DashboardDateFilter) -> String {
    match filter.preset {
        DashboardDatePreset::Live => "Динамика за последние 24 часа".to_string(),
        DashboardDatePreset::Today => "Динамика за сегодня".to_string(),
        DashboardDatePreset::Yesterday => "Динамика за вчера".to_string(),
        DashboardDatePreset::Custom => format!("Динамика за период {} — {}", filter.from_date, filter.to_date),
    }
}

// Режет диапазон на часы (до 48 часов) или на дни по локальному календарю.
pub fn timeline_buckets(range: &CamundaDateRange) -> Vec<TimelineBucket> {
    let hourly = (range.end - range.start).num_hours() <= 48;
    let mut cursor = if hourly { truncate_to_hour(range.start) } else { local_midnight(range.start.date_naive()) };
    let mut buckets = Vec::new();
    while cursor < range.end {
        let next = if hourly {
            cursor + Duration::hours(1)
        } else {
            match cursor.date_naive().succ_opt() {
                Some(next_day) => local_midnight(next_day),
                None => break,
            }
        };
        if next <= cursor {
            break;
        }
        let label = if hourly { cursor.format("%d.%m %H:%M") } else { cursor.format("%d.%m") }.to_string();
        buckets.push(TimelineBucket { label, start: cursor.with_timezone(&Utc), end: next.with_timezone(&Utc) });
        cursor = next;
    }
    buckets
}

pub fn is_inside(value: Option<&str>, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
    match parse_camunda_date_time(value) {
        Some(parsed) => {
            let instant = parsed.with_timezone(&Utc);
            instant >= start && instant < end
        }
        None => false,
    }
}

pub fn duration_millis(start: Option<&str>, end: Option<&str>) -> Option<i64> {
    let start = parse_camunda_date_time(start)?;
    let end = parse_camunda_date_time(end)?;
    let millis = (end - start).num_milliseconds();
    (millis >= 0).then_some(millis)
}

pub fn local_midnight(date: NaiveDate) -> DateTime<Local> {
    let naive = date.and_time(NaiveTime::MIN);
    Local.from_local_datetime(&naive).earliest().unwrap_or_else(|| Local.from_utc_datetime(&naive))
}

fn truncate_to_hour(value: DateTime<Local>) -> DateTime<Local> {
    value
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_camunda_format_and_rfc3339_fallback() {
        let camunda = parse_camunda_date_time(Some("2026-10-23T10:00:00.000+0500")).expect("дата Camunda");
        assert_eq!(camunda.to_rfc3339(), "2026-10-23T10:00:00+05:00");
        let rfc = parse_camunda_date_time(Some("2026-10-23T05:00:00Z")).expect("RFC 3339");
        assert_eq!(camunda.with_timezone(&Utc), rfc.with_timezone(&Utc));
        assert_eq!(parse_camunda_date_time(None), None);
        assert_eq!(parse_camunda_date_time(Some("  ")), None);
        assert_eq!(parse_camunda_date_time(Some("вчера")), None);
    }

    #[test]
    fn formats_offset_without_colon() {
        let value = FixedOffset::east_opt(5 * 3600)
            .and_then(|offset| offset.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).single())
            .expect("дата");
        assert_eq!(value.format(CAMUNDA_DATE_FORMAT).to_string(), "2026-10-23T10:00:00.000+0500");
    }

    #[test]
    fn custom_range_covers_whole_days_in_local_zone() {
        let filter = DashboardDateFilter {
            preset: DashboardDatePreset::Custom,
            from_date: "2026-07-15".to_string(),
            to_date: "2026-07-16".to_string(),
        };
        let range = camunda_date_range(&filter).expect("диапазон");
        assert_eq!(range.start.date_naive(), NaiveDate::from_ymd_opt(2026, 7, 15).expect("дата"));
        assert_eq!(range.end.date_naive(), NaiveDate::from_ymd_opt(2026, 7, 17).expect("дата"));
        assert!(range.after.starts_with("2026-07-14T23:59:59.999"));
        assert!(range.before.starts_with("2026-07-17T00:00:00.000"));
        assert_eq!(timeline_buckets(&range).len(), 48);
        assert!(timeline_buckets(&range)[0].label.starts_with("15.07 00:00"));
    }

    #[test]
    fn rejects_bad_dates_and_reversed_periods() {
        let bad = DashboardDateFilter {
            preset: DashboardDatePreset::Custom,
            from_date: "15.07.2026".to_string(),
            to_date: "2026-07-16".to_string(),
        };
        assert_eq!(camunda_date_range(&bad), Err("Введите дату в формате ГГГГ-ММ-ДД.".to_string()));
        let reversed = DashboardDateFilter {
            preset: DashboardDatePreset::Custom,
            from_date: "2026-07-17".to_string(),
            to_date: "2026-07-16".to_string(),
        };
        assert_eq!(camunda_date_range(&reversed), Err("Начальная дата не может быть позже конечной.".to_string()));
        assert!(camunda_date_range(&DashboardDateFilter::default()).is_err());
        assert_eq!(timeline_title(&DashboardDateFilter::default()), "Динамика за последние 24 часа");
    }

    #[test]
    fn long_periods_use_daily_buckets() {
        let filter = DashboardDateFilter {
            preset: DashboardDatePreset::Custom,
            from_date: "2026-07-01".to_string(),
            to_date: "2026-07-10".to_string(),
        };
        let buckets = timeline_buckets(&camunda_date_range(&filter).expect("диапазон"));
        assert_eq!(buckets.len(), 10);
        assert_eq!(buckets[0].label, "01.07");
        assert_eq!(buckets[9].label, "10.07");
    }

    #[test]
    fn computes_durations_and_membership() {
        assert_eq!(duration_millis(Some("2026-07-15T08:00:00.000+0800"), Some("2026-07-15T08:30:00.000+0800")), Some(1_800_000));
        assert_eq!(duration_millis(Some("2026-07-15T08:30:00.000+0800"), Some("2026-07-15T08:00:00.000+0800")), None);
        assert_eq!(duration_millis(Some("2026-07-15T08:30:00.000+0800"), None), None);
        let start = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).single().expect("дата");
        let end = start + Duration::hours(1);
        assert!(is_inside(Some("2026-07-15T08:00:00.000+0800"), start, end));
        assert!(!is_inside(Some("2026-07-15T09:00:00.000+0800"), start, end));
        assert!(!is_inside(None, start, end));
    }
}
