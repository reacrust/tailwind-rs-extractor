use tailwind_rs::TailwindBuilder;

/// Trait for processing Tailwind and custom classes.
/// 
/// This trait provides the shared logic for transforming class strings that may contain
/// a mix of Tailwind utility classes and custom CSS classes. It leverages the fact that
/// tailwind-rs's trace() method now passes through unrecognized classes unchanged,
/// greatly simplifying the processing logic.
pub trait TailwindClassProcessor {
    /// Get a reference to the TailwindBuilder for trace operations
    fn tailwind_builder(&mut self) -> &mut TailwindBuilder;
    
    /// Process a class string with Tailwind transformations.
    ///
    /// This method now simply calls trace() on the entire string, as trace()
    /// handles both Tailwind and custom classes correctly by passing through
    /// unrecognized classes unchanged.
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
        
        // Simply call trace() on the trimmed content - it now handles everything!
        // trace() will process Tailwind classes and pass through custom classes unchanged
        match self.tailwind_builder().trace(trimmed, obfuscate) {
            Ok(result) => {
                // Convert Cow to String - use into_owned() to get the String
                let result_str = result.into_owned();
                format!("{}{}{}", leading_space, result_str, trailing_space)
            },
            Err(_) => class_string.to_string(), // Fallback to original on error
        }
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
        
        // Custom class in the middle - trace() handles mixed classes
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

    #[test]
    fn test_process_with_fallback_preserves_mixed_class_order() {
        let mut processor = TestProcessor::new();
        
        // Test case 1: Custom-Tailwind-Custom-Tailwind-Custom pattern
        // trace() now handles mixed classes directly
        let input = "custom-a bg-blue-500 custom-b text-white custom-c";
        let result = processor.process_with_fallback(input, false);
        
        // Split result to analyze order
        let result_classes: Vec<&str> = result.split_whitespace().collect();
        
        // Find positions of custom classes (they should pass through unchanged)
        let custom_a_pos = result_classes.iter().position(|&c| c == "custom-a");
        let custom_b_pos = result_classes.iter().position(|&c| c == "custom-b");
        let custom_c_pos = result_classes.iter().position(|&c| c == "custom-c");
        
        assert!(custom_a_pos.is_some(), "custom-a should be present");
        assert!(custom_b_pos.is_some(), "custom-b should be present");
        assert!(custom_c_pos.is_some(), "custom-c should be present");
        
        // Verify order is preserved
        assert!(custom_a_pos.unwrap() < custom_b_pos.unwrap(), 
                "custom-a should come before custom-b");
        assert!(custom_b_pos.unwrap() < custom_c_pos.unwrap(), 
                "custom-b should come before custom-c");
        
        // Test case 2: More complex interleaving
        let input2 = "prefix-1 p-4 middle-1 bg-red-600 middle-2 text-lg suffix-1";
        let result2 = processor.process_with_fallback(input2, false);
        
        // Check that all custom classes appear in their original relative positions
        let pos_prefix = result2.find("prefix-1").expect("prefix-1 not found");
        let pos_middle1 = result2.find("middle-1").expect("middle-1 not found");
        let pos_middle2 = result2.find("middle-2").expect("middle-2 not found");
        let pos_suffix = result2.find("suffix-1").expect("suffix-1 not found");
        
        assert!(pos_prefix < pos_middle1, "prefix-1 should come before middle-1");
        assert!(pos_middle1 < pos_middle2, "middle-1 should come before middle-2");
        assert!(pos_middle2 < pos_suffix, "middle-2 should come before suffix-1");
        
        // Test case 3: Verify the exact output structure
        // Even if Tailwind classes are transformed, the overall sequence should be maintained
        let input3 = "my-custom bg-blue-500 another-custom";
        let result3 = processor.process_with_fallback(input3, false);
        let result3_classes: Vec<&str> = result3.split_whitespace().collect();
        
        // First class should start with "my-custom" or be "my-custom"
        assert_eq!(result3_classes[0], "my-custom", "First class should be my-custom");
        
        // Last class should be "another-custom"
        assert_eq!(result3_classes[result3_classes.len() - 1], "another-custom", 
                   "Last class should be another-custom");
        
        // Middle content should be the processed Tailwind class(es)
        // The key is that custom classes maintain their positions
    }
    
    #[test]
    fn test_exact_order_preservation_with_mixed_classes() {
        let mut processor = TestProcessor::new();
        
        // This test verifies that when custom classes are interspersed with Tailwind classes,
        // the trace() method maintains their exact order
        
        // Pattern with custom classes in positions 0, 2, 4
        let input = "custom-first p-4 custom-second bg-blue-500 custom-third text-white custom-fourth";
        let result = processor.process_with_fallback(input, false);
        
        // Parse result into individual classes
        let input_classes: Vec<&str> = input.split_whitespace().collect();
        let result_classes: Vec<&str> = result.split_whitespace().collect();
        
        // The number of classes should be the same (no combining or splitting)
        assert_eq!(input_classes.len(), result_classes.len(), 
                   "Number of classes should remain the same");
        
        // Verify each custom class appears at its original index
        assert_eq!(result_classes[0], "custom-first", 
                   "Position 0: custom-first should remain at index 0");
        assert_eq!(result_classes[2], "custom-second", 
                   "Position 2: custom-second should remain at index 2");
        assert_eq!(result_classes[4], "custom-third", 
                   "Position 4: custom-third should remain at index 4");
        assert_eq!(result_classes[6], "custom-fourth", 
                   "Position 6: custom-fourth should remain at index 6");
        
        // Tailwind classes should be at their original positions (transformed or not)
        // They should be at positions 1, 3, 5
        // We don't check their exact values as they might be transformed,
        // but they should exist at these positions
        assert!(!result_classes[1].is_empty(), "Position 1 should have content (Tailwind class)");
        assert!(!result_classes[3].is_empty(), "Position 3 should have content (Tailwind class)");
        assert!(!result_classes[5].is_empty(), "Position 5 should have content (Tailwind class)");
        
        // Comprehensive order verification: compare each pair
        for i in 0..input_classes.len() {
            // If it's a custom class in the input, it should be unchanged in the output
            if input_classes[i].starts_with("custom-") {
                assert_eq!(result_classes[i], input_classes[i], 
                           "Custom class at position {} should be unchanged", i);
            }
        }
    }
}