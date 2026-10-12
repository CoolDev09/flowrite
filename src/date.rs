pub(super) fn today(format: usize) -> String {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millisecond: u16,
    }
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLocalTime(time: *mut SystemTime);
        }
        let mut date = SystemTime::default();
        unsafe {
            GetLocalTime(&mut date);
        }
        return date_string(date.year, date.month, date.day, format);
    }
    #[cfg(not(windows))]
    {
        date_string(2026, 10, 10, format)
    }
}
fn date_string(year: u16, month: u16, day: u16, format: usize) -> String {
    let name = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ][month.saturating_sub(1).min(11) as usize];
    match format {
        1 => format!("{name} {day}, {year}"),
        2 => format!("{month:02}/{day:02}/{year}"),
        _ => format!("{day} {name} {year}"),
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn heading_date_formats() {
        assert_eq!(super::date_string(2026, 10, 10, 0), "10 October 2026");
        assert_eq!(super::date_string(2026, 10, 10, 1), "October 10, 2026");
    }
}
