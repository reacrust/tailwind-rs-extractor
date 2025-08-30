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
    /// # Arguments
    /// * `class_string` - The class string to process (space-separated classes)
    /// * `obfuscate` - Whether to obfuscate Tailwind classes for production
    ///
    /// # Returns
    /// The processed class string with Tailwind transformations applied
    fn process_with_fallback(&mut self, class_string: &str, obfuscate: bool) -> String {
        // First try the whole string - optimal for pure Tailwind classes
        if let Ok(result) = self.tailwind_builder().trace(class_string, obfuscate) {
            return result;
        }

        // Split into individual classes
        let classes: Vec<&str> = class_string.split_whitespace().collect();
        
        // If empty or single class, return as-is
        if classes.len() <= 1 {
            return class_string.to_string();
        }

        // Try excluding the first class (maybe it's the only custom one)
        if classes.len() > 1 {
            let without_first = classes[1..].join(" ");
            if let Ok(traced_tail) = self.tailwind_builder().trace(&without_first, obfuscate) {
                return format!("{} {}", classes[0], traced_tail);
            }
        }

        // Try excluding the last class (maybe it's the only custom one)
        if classes.len() > 1 {
            let without_last = classes[..classes.len() - 1].join(" ");
            if let Ok(traced_head) = self.tailwind_builder().trace(&without_last, obfuscate) {
                return format!("{} {}", traced_head, classes[classes.len() - 1]);
            }
        }

        // Last resort: try each class individually
        let processed: Vec<String> = classes.iter().map(|class| {
            match self.tailwind_builder().trace(class, obfuscate) {
                Ok(traced) => traced,
                Err(_) => class.to_string(), // Pass through non-Tailwind classes unchanged
            }
        }).collect();
        
        processed.join(" ")
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
}