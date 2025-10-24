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
    // TODO: Use chrono or time crate for proper timestamp formatting
    format!("{} ms (Unix epoch)", millis)
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
