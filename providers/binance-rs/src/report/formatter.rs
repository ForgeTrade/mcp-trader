// Markdown formatting utilities for report generation

/// Build a markdown table from headers and rows
pub fn build_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = String::new();

    // Header row
    table.push_str("| ");
    table.push_str(&headers.join(" | "));
    table.push_str(" |\n");

    // Separator row
    table.push_str("|");
    for _ in headers {
        table.push_str("--------|");
    }
    table.push('\n');

    // Data rows
    for row in rows {
        table.push_str("| ");
        table.push_str(&row.join(" | "));
        table.push_str(" |\n");
    }

    table
}

/// Build a markdown list from items
pub fn build_list(items: &[String], ordered: bool) -> String {
    let mut list = String::new();

    for (i, item) in items.iter().enumerate() {
        if ordered {
            list.push_str(&format!("{}. {}\n", i + 1, item));
        } else {
            list.push_str(&format!("- {}\n", item));
        }
    }

    list
}

/// Build a markdown section header
pub fn build_section_header(title: &str, level: u8) -> String {
    let hashes = "#".repeat(level as usize);
    format!("{} {}\n\n", hashes, title)
}

/// Format a price with thousand separators and tick size precision
///
/// For BTCUSDT (tick_size = 0.01): 113559.99 -> "113 559.99"
/// For other pairs: adjusts decimals based on tick_size
pub fn format_price(price: &str, decimals: usize) -> String {
    // Parse price string to f64
    let price_f64: f64 = match price.parse() {
        Ok(p) => p,
        Err(_) => return price.to_string(), // Return as-is if parse fails
    };

    // Format with specified decimals
    let formatted = format!("{:.prec$}", price_f64, prec = decimals);

    // Split into integer and fractional parts
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts.get(0).unwrap_or(&"0");
    let fractional_part = parts.get(1);

    // Add thousand separators to integer part (space as separator)
    let mut integer_with_separators = String::new();
    let chars: Vec<char> = integer_part.chars().collect();
    let len = chars.len();

    for (i, ch) in chars.iter().enumerate() {
        integer_with_separators.push(*ch);
        // Add space every 3 digits from the right (but not at the end)
        if (len - i - 1) % 3 == 0 && i < len - 1 {
            integer_with_separators.push(' ');
        }
    }

    // Combine integer and fractional parts
    match fractional_part {
        Some(frac) => format!("{}.{}", integer_with_separators, frac),
        None => integer_with_separators,
    }
}

/// Format a price (f64) with thousand separators and precision
///
/// For high-precision values like Mid Price: 113516.985 -> "113 516.98500" (5 decimals)
pub fn format_price_f64(price: f64, decimals: usize) -> String {
    // Format with specified decimals
    let formatted = format!("{:.prec$}", price, prec = decimals);

    // Split into integer and fractional parts
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts.get(0).unwrap_or(&"0");
    let fractional_part = parts.get(1);

    // Add thousand separators to integer part (space as separator)
    let mut integer_with_separators = String::new();
    let chars: Vec<char> = integer_part.chars().collect();
    let len = chars.len();

    for (i, ch) in chars.iter().enumerate() {
        integer_with_separators.push(*ch);
        // Add space every 3 digits from the right (but not at the end)
        if (len - i - 1) % 3 == 0 && i < len - 1 {
            integer_with_separators.push(' ');
        }
    }

    // Combine integer and fractional parts
    match fractional_part {
        Some(frac) => format!("{}.{}", integer_with_separators, frac),
        None => integer_with_separators,
    }
}

/// Format large USD amounts with B/M suffixes
///
/// Examples:
/// - 1_139_664_263.27 -> "$1.14B"
/// - 5_500_000.50 -> "$5.50M"
/// - 999_999.99 -> "$1.00M"
pub fn format_large_usd(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        // Billions
        format!("${:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        // Millions
        format!("${:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        // Thousands
        format!("${:.2}K", value / 1_000.0)
    } else {
        // Below 1K, show 2 decimals
        format!("${:.2}", value)
    }
}

/// Format a percentage value
pub fn format_percentage(value: f64) -> String {
    format!("{:.2}%", value)
}

/// Format a currency value
pub fn format_currency(value: f64, decimals: usize) -> String {
    format!("${:.prec$}", value, prec = decimals)
}

/// Format a timestamp as ISO 8601 UTC
pub fn format_timestamp(millis: i64) -> String {
    use chrono::{DateTime, TimeZone, Utc};

    match Utc.timestamp_millis_opt(millis) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        _ => format!("{} ms (Unix epoch)", millis),
    }
}

/// Format a DateTime<Utc> as human-readable string
pub fn format_datetime(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_table() {
        let headers = vec!["Name", "Value"];
        let rows = vec![
            vec!["Price".to_string(), "$42,000".to_string()],
            vec!["Volume".to_string(), "1,234 BTC".to_string()],
        ];

        let table = build_table(&headers, &rows);
        assert!(table.contains("| Name | Value |"));
        assert!(table.contains("| Price | $42,000 |"));
    }

    #[test]
    fn test_build_list() {
        let items = vec!["First".to_string(), "Second".to_string()];

        let unordered = build_list(&items, false);
        assert!(unordered.contains("- First"));

        let ordered = build_list(&items, true);
        assert!(ordered.contains("1. First"));
    }

    #[test]
    fn test_build_section_header() {
        assert_eq!(build_section_header("Title", 2), "## Title\n\n");
        assert_eq!(build_section_header("Subtitle", 3), "### Subtitle\n\n");
    }
}
