pub fn get_pay_i64(s: &str) -> Result<i64, String> {
    if let Ok(num) = s.parse::<f64>() {
        return Ok((num * 100.0).round() as i64);
    }

    Err("Invalid input string".to_string())
}

pub fn get_pay_str(num: Option<i64>) -> String {
    match num {
        Some(num) => format!("{:.2}", num as f64 / 100.0),
        None => "".to_string(),
    }
}

pub fn format_comma_separated(str: String) -> String {
    str.split(',')
        .map(|s| {
            let mut chars = s.trim().chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn format_location(city: &str, region: &str, country: &str) -> String {
    [city, region, country]
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn total_pages(total_items: i64, page_size: i64) -> i64 {
    (total_items + page_size - 1) / page_size
}
