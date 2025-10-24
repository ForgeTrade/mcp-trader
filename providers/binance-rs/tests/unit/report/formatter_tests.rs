// Unit tests for markdown formatter utilities

use binance_provider::report::formatter::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Note: formatter.rs already includes basic tests
    // These tests cover additional edge cases:

    #[test]
    fn test_empty_table() {
        let headers = vec!["Column 1", "Column 2"];
        let rows: Vec<Vec<String>> = vec![];

        let table = build_table(&headers, &rows);

        // Should still have header and separator
        assert!(table.contains("| Column 1 | Column 2 |"));
        assert!(table.contains("|--------|--------|"));

        // But no data rows
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 2); // Only header + separator
    }

    #[test]
    fn test_table_with_special_characters() {
        let headers = vec!["Name", "Value"];
        let rows = vec![
            vec!["Price | Volume".to_string(), "$42,000 *".to_string()],
            vec!["Hash#123".to_string(), "100%".to_string()],
        ];

        let table = build_table(&headers, &rows);

        // Special characters should be preserved (no escaping in this simple implementation)
        assert!(table.contains("| Price | Volume |"));
        assert!(table.contains("| Hash#123 | 100% |"));
    }

    #[test]
    fn test_very_long_list() {
        // Generate list with 150 items
        let items: Vec<String> = (1..=150).map(|i| format!("Item {}", i)).collect();

        let unordered = build_list(&items, false);
        let ordered = build_list(&items, true);

        // Verify first and last items
        assert!(unordered.starts_with("- Item 1\n"));
        assert!(unordered.contains("- Item 150\n"));

        assert!(ordered.starts_with("1. Item 1\n"));
        assert!(ordered.contains("150. Item 150\n"));

        // Count lines
        let unordered_lines: Vec<&str> = unordered.lines().collect();
        let ordered_lines: Vec<&str> = ordered.lines().collect();
        assert_eq!(unordered_lines.len(), 150);
        assert_eq!(ordered_lines.len(), 150);
    }

    #[test]
    fn test_format_percentage_edge_cases() {
        // Zero
        assert_eq!(format_percentage(0.0), "0.00%");

        // Negative values
        assert_eq!(format_percentage(-5.5), "-5.50%");
        assert_eq!(format_percentage(-100.0), "-100.00%");

        // Very large values
        assert_eq!(format_percentage(1000.0), "1000.00%");
        assert_eq!(format_percentage(999999.99), "999999.99%");

        // Very small positive
        assert_eq!(format_percentage(0.01), "0.01%");
        assert_eq!(format_percentage(0.001), "0.00%"); // Rounds to 2 decimals
    }

    #[test]
    fn test_format_currency_precision() {
        // 0 decimals (whole numbers)
        assert_eq!(format_currency(42000.0, 0), "$42000");

        // 2 decimals (standard)
        assert_eq!(format_currency(42000.50, 2), "$42000.50");
        assert_eq!(format_currency(0.01, 2), "$0.01");

        // High precision (8 decimals for crypto)
        assert_eq!(format_currency(0.00012345, 8), "$0.00012345");
        assert_eq!(format_currency(123.456789, 8), "$123.45678900");

        // Negative values
        assert_eq!(format_currency(-100.0, 2), "$-100.00");
    }

    #[test]
    fn test_build_section_header_edge_cases() {
        // Level 1 (single #)
        assert_eq!(build_section_header("Main Title", 1), "# Main Title\n\n");

        // Level 6 (max markdown header)
        assert_eq!(
            build_section_header("Deep Section", 6),
            "###### Deep Section\n\n"
        );

        // Empty title
        assert_eq!(build_section_header("", 2), "## \n\n");

        // Title with special characters
        assert_eq!(
            build_section_header("Price & Volume", 2),
            "## Price & Volume\n\n"
        );
    }

    #[test]
    fn test_build_list_empty() {
        let items: Vec<String> = vec![];

        let unordered = build_list(&items, false);
        let ordered = build_list(&items, true);

        assert_eq!(unordered, "");
        assert_eq!(ordered, "");
    }

    #[test]
    fn test_build_list_single_item() {
        let items = vec!["Only item".to_string()];

        let unordered = build_list(&items, false);
        let ordered = build_list(&items, true);

        assert_eq!(unordered, "- Only item\n");
        assert_eq!(ordered, "1. Only item\n");
    }
}
