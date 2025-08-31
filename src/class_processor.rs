use tailwind_rs::TailwindBuilder;

/// Trait for processing Tailwind and custom classes with intelligent fallback strategies.
/// 
/// This trait provides the shared logic for transforming class strings that may contain
/// a mix of Tailwind utility classes and custom CSS classes. It implements a 4-tier
/// fallback strategy to handle all possible combinations while preserving the order
/// and integrity of custom classes.
pub trait TailwindClassProcessor {
    /// Get a reference to the TailwindBuilder for trace operations
    fn tailwind_builder(&mut self) -> &mut TailwindBuilder;
    
    /// Process a class string with intelligent fallback handling for mixed classes.
    ///
    /// This method implements a 4-tier fallback strategy:
    /// 1. Try the whole string as pure Tailwind classes
    /// 2. Try excluding the first class (custom prefix + Tailwind)
    /// 3. Try excluding the last class (Tailwind + custom suffix) 
    /// 4. Process each class individually (mixed Tailwind and custom)
    ///
    /// IMPORTANT: This method preserves leading and trailing whitespace to ensure
    /// proper concatenation in JavaScript string expressions.
    ///
    /// # Arguments
    /// * `class_string` - The class string to process (space-separated classes)
    /// * `obfuscate` - Whether to obfuscate Tailwind classes for production
    ///
    /// # Returns
    /// The processed class string with Tailwind transformations applied
    fn process_with_fallback(&mut self, class_string: &str, obfuscate: bool) -> String {
        // Work with trimmed string for processing
        let trimmed = class_string.trim();
        
        // If empty after trimming, return with preserved spaces
        if trimmed.is_empty() {
            return class_string.to_string();
        }
        
        // Detect and preserve ALL leading/trailing whitespace
        // Find where the actual content starts and ends
        let content_start = class_string.find(|c: char| !c.is_whitespace()).unwrap_or(0);
        let content_end = class_string.rfind(|c: char| !c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(class_string.len());
        
        let leading_space = &class_string[..content_start];
        let trailing_space = &class_string[content_end..];
        
        // First try the whole string - optimal for pure Tailwind classes
        if let Ok(result) = self.tailwind_builder().trace(trimmed, obfuscate) {
            return format!("{}{}{}", leading_space, result, trailing_space);
        }

        // Split into individual classes
        let classes: Vec<&str> = trimmed.split_whitespace().collect();
        
        // If single class, try to process it
        if classes.len() == 1 {
            match self.tailwind_builder().trace(classes[0], obfuscate) {
                Ok(traced) => return format!("{}{}{}", leading_space, traced, trailing_space),
                Err(_) => return class_string.to_string(), // Return original with spaces
            }
        }

        // Try excluding the first class (maybe it's the only custom one)
        if classes.len() > 1 {
            let without_first = classes[1..].join(" ");
            if let Ok(traced_tail) = self.tailwind_builder().trace(&without_first, obfuscate) {
                return format!("{}{} {}{}", leading_space, classes[0], traced_tail, trailing_space);
            }
        }

        // Try excluding the last class (maybe it's the only custom one)
        if classes.len() > 1 {
            let without_last = classes[..classes.len() - 1].join(" ");
            if let Ok(traced_head) = self.tailwind_builder().trace(&without_last, obfuscate) {
                return format!("{}{} {}{}", leading_space, traced_head, classes[classes.len() - 1], trailing_space);
            }
        }

        // Last resort: try each class individually
        // ALWAYS process all classes, even if they don't look like Tailwind
        let processed: Vec<String> = classes.iter().map(|class| {
            match self.tailwind_builder().trace(class, obfuscate) {
                Ok(traced) => traced,
                Err(_) => class.to_string(), // Pass through non-Tailwind classes unchanged
            }
        }).collect();
        
        format!("{}{}{}", leading_space, processed.join(" "), trailing_space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock implementation for testing
    struct TestProcessor {
        builder: TailwindBuilder,
    }

    impl TestProcessor {
        fn new() -> Self {
            Self {
                builder: TailwindBuilder::default(),
            }
        }
    }

    impl TailwindClassProcessor for TestProcessor {
        fn tailwind_builder(&mut self) -> &mut TailwindBuilder {
            &mut self.builder
        }
    }

    #[test]
    fn test_process_with_fallback_all_custom_classes() {
        let mut processor = TestProcessor::new();
        
        // Test with all custom classes (non-Tailwind)
        let result = processor.process_with_fallback("my-custom-class another-class test-component", false);
        assert_eq!(result, "my-custom-class another-class test-component");
    }

    #[test]
    fn test_process_with_fallback_all_tailwind_classes() {
        let mut processor = TestProcessor::new();
        
        // Test with valid Tailwind classes
        let result = processor.process_with_fallback("p-4 bg-blue-500 text-white", false);
        
        // Should process successfully (actual output depends on tailwind-rs implementation)
        assert!(!result.is_empty());
    }

    #[test]
    fn test_process_with_fallback_mixed_custom_first() {
        let mut processor = TestProcessor::new();
        
        // Custom class at the beginning
        let result = processor.process_with_fallback("my-custom-class bg-blue-500 text-white", false);
        
        // The custom class should be preserved at the beginning
        assert!(result.starts_with("my-custom-class"));
    }

    #[test]
    fn test_process_with_fallback_mixed_custom_last() {
        let mut processor = TestProcessor::new();
        
        // Custom class at the end
        let result = processor.process_with_fallback("bg-blue-500 text-white my-custom-class", false);
        
        // The custom class should be preserved at the end
        assert!(result.ends_with("my-custom-class"));
    }

    #[test]
    fn test_process_with_fallback_mixed_custom_middle() {
        let mut processor = TestProcessor::new();
        
        // Custom class in the middle - will fall back to individual processing
        let result = processor.process_with_fallback("bg-blue-500 my-custom-class text-white", false);
        
        // All classes should be preserved
        assert!(result.contains("my-custom-class"));
    }

    #[test]
    fn test_process_with_fallback_single_class() {
        let mut processor = TestProcessor::new();
        
        // Single custom class
        let result = processor.process_with_fallback("single-class", false);
        assert_eq!(result, "single-class");
        
        // Single Tailwind class
        let result = processor.process_with_fallback("p-4", false);
        assert!(!result.is_empty()); // Should be processed or returned as-is
    }

    #[test]
    fn test_process_with_fallback_empty() {
        let mut processor = TestProcessor::new();
        
        let result = processor.process_with_fallback("", false);
        assert_eq!(result, "");
    }

    #[test]
    fn test_process_with_fallback_whitespace_handling() {
        let mut processor = TestProcessor::new();
        
        // Multiple spaces between classes
        let result = processor.process_with_fallback("class1    class2     class3", false);
        
        // Should normalize to single spaces
        let parts: Vec<&str> = result.split_whitespace().collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_process_with_fallback_preserves_order() {
        let mut processor = TestProcessor::new();
        
        // Mixed classes should preserve their relative order
        let result = processor.process_with_fallback("first-custom bg-blue-500 middle-custom text-white last-custom", false);
        
        // Check that custom classes appear in order (even if processing changes Tailwind classes)
        let first_pos = result.find("first-custom").expect("first-custom not found");
        let middle_pos = result.find("middle-custom").expect("middle-custom not found");
        let last_pos = result.find("last-custom").expect("last-custom not found");
        
        assert!(first_pos < middle_pos, "Order not preserved: first should come before middle");
        assert!(middle_pos < last_pos, "Order not preserved: middle should come before last");
    }

    #[test]
    fn test_process_with_fallback_with_obfuscation_all_tailwind() {
        let mut processor = TestProcessor::new();
        
        // With obfuscation, all Tailwind classes might get combined into a single token
        let result = processor.process_with_fallback("p-4 bg-blue-500 text-white", true);
        
        // Result should be non-empty and potentially shorter than input if obfuscated
        assert!(!result.is_empty());
    }

    #[test]
    fn test_process_with_fallback_with_obfuscation_mixed() {
        let mut processor = TestProcessor::new();
        
        // Custom class at beginning with obfuscation
        let result = processor.process_with_fallback("my-custom-class bg-blue-500 text-white", true);
        assert!(result.starts_with("my-custom-class"), "Custom class should be preserved at start");
        
        // Custom class at end with obfuscation
        let result = processor.process_with_fallback("bg-blue-500 text-white my-custom-class", true);
        assert!(result.ends_with("my-custom-class"), "Custom class should be preserved at end");
    }

    #[test]
    fn test_process_with_fallback_obfuscation_preserves_custom() {
        let mut processor = TestProcessor::new();
        
        // All custom classes should pass through unchanged even with obfuscation
        let result = processor.process_with_fallback("custom1 custom2 custom3", true);
        assert_eq!(result, "custom1 custom2 custom3", "Custom classes should not be obfuscated");
    }

    #[test]
    fn test_process_with_fallback_obfuscation_fallback() {
        let mut processor = TestProcessor::new();
        
        // When falling back to individual processing with obfuscation,
        // custom classes should still be preserved
        let result = processor.process_with_fallback("bg-blue-500 my-custom text-white another-custom p-4", true);
        
        assert!(result.contains("my-custom"), "First custom class should be preserved");
        assert!(result.contains("another-custom"), "Second custom class should be preserved");
        
        // Check order is maintained
        let custom1_pos = result.find("my-custom").expect("my-custom not found");
        let custom2_pos = result.find("another-custom").expect("another-custom not found");
        assert!(custom1_pos < custom2_pos, "Custom class order should be preserved");
    }

    // ========== Comprehensive Space Preservation Tests ==========

    #[test]
    fn test_preserves_leading_space() {
        let mut processor = TestProcessor::new();
        
        // Single leading space
        let result = processor.process_with_fallback(" p-4 bg-blue-500", false);
        assert!(result.starts_with(" "), "Should preserve single leading space");
        
        // Multiple leading spaces
        let result = processor.process_with_fallback("   custom-class text-white", false);
        assert!(result.starts_with("   "), "Should preserve multiple leading spaces");
    }

    #[test]
    fn test_preserves_trailing_space() {
        let mut processor = TestProcessor::new();
        
        // Single trailing space
        let result = processor.process_with_fallback("p-4 bg-blue-500 ", false);
        assert!(result.ends_with(" "), "Should preserve single trailing space");
        
        // Multiple trailing spaces
        let result = processor.process_with_fallback("custom-class text-white   ", false);
        assert!(result.ends_with("   "), "Should preserve multiple trailing spaces");
    }

    #[test]
    fn test_preserves_both_leading_and_trailing_spaces() {
        let mut processor = TestProcessor::new();
        
        // Single space on both ends
        let result = processor.process_with_fallback(" p-4 bg-blue-500 ", false);
        assert!(result.starts_with(" "), "Should preserve leading space");
        assert!(result.ends_with(" "), "Should preserve trailing space");
        assert!(result.contains("p-4"), "Should preserve content");
        
        // Multiple spaces on both ends
        let result = processor.process_with_fallback("  custom-class text-white  ", false);
        assert!(result.starts_with("  "), "Should preserve multiple leading spaces");
        assert!(result.ends_with("  "), "Should preserve multiple trailing spaces");
    }

    #[test]
    fn test_preserves_spaces_with_single_class() {
        let mut processor = TestProcessor::new();
        
        // Leading space with single class
        let result = processor.process_with_fallback(" single-class", false);
        assert_eq!(result, " single-class");
        
        // Trailing space with single class
        let result = processor.process_with_fallback("single-class ", false);
        assert_eq!(result, "single-class ");
        
        // Both spaces with single class
        let result = processor.process_with_fallback(" single-class ", false);
        assert_eq!(result, " single-class ");
    }

    #[test]
    fn test_empty_string_with_spaces() {
        let mut processor = TestProcessor::new();
        
        // Only spaces (no classes)
        let result = processor.process_with_fallback("   ", false);
        assert_eq!(result, "   ", "Should preserve spaces even with no classes");
        
        // Single space
        let result = processor.process_with_fallback(" ", false);
        assert_eq!(result, " ", "Should preserve single space");
        
        // Many spaces
        let result = processor.process_with_fallback("     ", false);
        assert_eq!(result, "     ", "Should preserve multiple spaces");
    }

    #[test]
    fn test_mixed_tailwind_and_custom_with_spaces() {
        let mut processor = TestProcessor::new();
        
        // Leading space with mixed classes
        let result = processor.process_with_fallback(" custom-class p-4 bg-blue-500", false);
        assert!(result.starts_with(" "), "Should preserve leading space with mixed classes");
        assert!(result.contains("custom-class"), "Should preserve custom class");
        
        // Trailing space with mixed classes
        let result = processor.process_with_fallback("p-4 custom-class bg-blue-500 ", false);
        assert!(result.ends_with(" "), "Should preserve trailing space with mixed classes");
        assert!(result.contains("custom-class"), "Should preserve custom class");
        
        // Both spaces with mixed classes
        let result = processor.process_with_fallback("  bg-blue-500 custom-class p-4  ", false);
        assert!(result.starts_with("  "), "Should preserve leading spaces");
        assert!(result.ends_with("  "), "Should preserve trailing spaces");
        assert!(result.contains("custom-class"), "Should preserve custom class");
    }

    #[test]
    fn test_spaces_with_obfuscation() {
        let mut processor = TestProcessor::new();
        
        // Leading space with obfuscation
        let result = processor.process_with_fallback(" p-4 bg-blue-500", true);
        assert!(result.starts_with(" "), "Should preserve leading space with obfuscation");
        
        // Trailing space with obfuscation
        let result = processor.process_with_fallback("p-4 bg-blue-500 ", true);
        assert!(result.ends_with(" "), "Should preserve trailing space with obfuscation");
        
        // Both spaces with obfuscation
        let result = processor.process_with_fallback(" p-4 bg-blue-500 ", true);
        assert!(result.starts_with(" ") && result.ends_with(" "), 
                "Should preserve both spaces with obfuscation");
    }

    #[test]
    fn test_edge_cases_with_spaces() {
        let mut processor = TestProcessor::new();
        
        // Tab characters (should be preserved as-is since we only trim spaces)
        let result = processor.process_with_fallback("\tp-4 bg-blue-500\t", false);
        assert!(result.starts_with("\t"), "Should preserve tab at start");
        assert!(result.ends_with("\t"), "Should preserve tab at end");
        
        // Newlines (should be preserved)
        let result = processor.process_with_fallback("\np-4 bg-blue-500\n", false);
        assert!(result.starts_with("\n"), "Should preserve newline at start");
        assert!(result.ends_with("\n"), "Should preserve newline at end");
        
        // Mixed whitespace (only spaces should be counted for preservation)
        let result = processor.process_with_fallback(" \tp-4 bg-blue-500 \n", false);
        assert!(result.starts_with(" \t"), "Should preserve mixed whitespace at start");
        assert!(result.ends_with(" \n"), "Should preserve mixed whitespace at end");
    }

    #[test]
    fn test_internal_space_normalization() {
        let mut processor = TestProcessor::new();
        
        // Multiple spaces between classes should be normalized to single space
        let result = processor.process_with_fallback(" class1    class2     class3 ", false);
        assert!(result.starts_with(" "), "Should preserve leading space");
        assert!(result.ends_with(" "), "Should preserve trailing space");
        
        // Check internal normalization
        let trimmed = result.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        assert_eq!(parts.len(), 3, "Should have 3 classes after normalization");
        
        // Verify the internal structure has single spaces
        assert!(trimmed.contains("class1") && trimmed.contains("class2") && trimmed.contains("class3"));
        let normalized_internal = parts.join(" ");
        assert_eq!(trimmed, normalized_internal, "Internal spaces should be normalized");
    }

    #[test]
    fn test_extreme_space_cases() {
        let mut processor = TestProcessor::new();
        
        // Very long leading/trailing spaces
        let long_spaces = " ".repeat(10);
        let input = format!("{}p-4{}", &long_spaces, &long_spaces);
        let result = processor.process_with_fallback(&input, false);
        assert!(result.starts_with(&long_spaces), "Should preserve long leading spaces");
        assert!(result.ends_with(&long_spaces), "Should preserve long trailing spaces");
        
        // Empty string (no spaces)
        let result = processor.process_with_fallback("", false);
        assert_eq!(result, "", "Empty string should remain empty");
        
        // Only internal spaces (no leading/trailing)
        let result = processor.process_with_fallback("class1    class2", false);
        assert!(!result.starts_with(" "), "Should not add leading space");
        assert!(!result.ends_with(" "), "Should not add trailing space");
    }

    #[test]
    fn test_spaces_preserved_after_processing_errors() {
        let mut processor = TestProcessor::new();
        
        // Even if processing fails, spaces should be preserved
        // Using classes that might fail processing but should still preserve spaces
        let result = processor.process_with_fallback("  invalid!@# class$%^ ", false);
        assert!(result.starts_with("  "), "Should preserve leading spaces even with invalid classes");
        assert!(result.ends_with(" "), "Should preserve trailing space even with invalid classes");
        assert!(result.contains("invalid!@#"), "Should preserve invalid class names");
        assert!(result.contains("class$%^"), "Should preserve invalid class names");
    }

    #[test]
    fn test_multiple_spaces_at_beginning_and_end() {
        let mut processor = TestProcessor::new();
        
        // Test with 2, 3, 4, 5 spaces
        for num_spaces in 2..=5 {
            let spaces = " ".repeat(num_spaces);
            let input = format!("{}text-white{}", &spaces, &spaces);
            let result = processor.process_with_fallback(&input, false);
            
            assert!(result.starts_with(&spaces), 
                    "Should preserve {} leading spaces", num_spaces);
            assert!(result.ends_with(&spaces), 
                    "Should preserve {} trailing spaces", num_spaces);
        }
    }

    #[test]
    fn test_whitespace_only_strings() {
        let mut processor = TestProcessor::new();
        
        // Different amounts of whitespace-only strings
        for num_spaces in 1..=10 {
            let input = " ".repeat(num_spaces);
            let result = processor.process_with_fallback(&input, false);
            assert_eq!(result, input, 
                       "Should preserve whitespace-only string with {} spaces", num_spaces);
        }
    }

    #[test]
    fn test_normal_case_no_spaces() {
        let mut processor = TestProcessor::new();
        
        // No leading or trailing spaces - the normal case
        let result = processor.process_with_fallback("p-4 bg-blue-500 text-white", false);
        assert!(!result.starts_with(" "), "Should not add leading space when not present");
        assert!(!result.ends_with(" "), "Should not add trailing space when not present");
        
        // With custom classes
        let result = processor.process_with_fallback("custom-class another-custom", false);
        assert_eq!(result, "custom-class another-custom", 
                   "Should return unchanged when no spaces at edges");
    }
}