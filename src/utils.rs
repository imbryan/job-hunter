use regex::Regex;

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

pub fn parse_salary(salary_str: &str) -> Vec<(f64, String)> {
    let re = Regex::new(r"\D([\d,]+\.\d\d)\/([a-z]*)").expect("Failed to make regex");
    let mut results = Vec::new();
    for cap in re.captures_iter(salary_str) {
        let no = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let cleaned = no.replace(",", "");

        if let Ok(no_f64) = cleaned.parse::<f64>() {
            let pay_freq = cap.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
            results.push((no_f64, pay_freq));
        }
    }
    results
}

pub fn find_yoe_naive(text: &str) -> (Option<i64>, Option<i64>) {
    let re = Regex::new(r"([\d]*)[\D]?([\d]+)[\+]? year[s]?").expect("Failed to make regex");
    let mut min_yoe = i64::MAX;
    let mut max_yoe = i64::MIN;
    for cap in re.captures_iter(text) {
        for group in cap.iter() {
            if let Ok(num) = group.map(|m| m.as_str()).unwrap_or("").parse::<i64>() {
                if num < min_yoe {
                    min_yoe = num;
                }
                if num > max_yoe {
                    max_yoe = num;
                }
            }
        }
    }
    let mut results = (None, None);
    if min_yoe < i64::MAX {
        results.0 = Some(min_yoe);
    }
    if max_yoe > i64::MIN && max_yoe != min_yoe {
        results.1 = Some(max_yoe);
    }
    results
}
