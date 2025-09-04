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
        // trace() will process Tailwind classes and pass through custom classes unchanged
        match self.tailwind_builder().trace(class_string, obfuscate) {
            Ok(result) =>  result.into_owned(),
            Err(_) => class_string.to_string(), // Fallback to original on error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_process_preserves_whitespace_after_transition_colors() {
        let mut processor = TestProcessor::new();
        
        let input = "px-4 py-2 rounded-lg font-[500] transition-colors text-gray-600 hover:bg-gray-100";
        let result = processor.process_with_fallback(input, false);
        
        // The bug would cause "transition-colorstext-gray-600" (missing space)
        assert!(
            result.contains("transition-colors text-gray-600"),
            "Space should be preserved between 'transition-colors' and 'text-gray-600'. Got: '{}'",
            result
        );
        
        // Ensure no classes are concatenated
        assert!(
            !result.contains("transition-colorstext"),
            "Classes should not be concatenated. Found 'transition-colorstext' in: '{}'", 
            result
        );
    }

    #[test]
    fn test_process_preserves_all_spaces() {
        let mut processor = TestProcessor::new();
        
        let test_cases = vec![
            "transition-colors text-gray-600",
            "font-bold transition-colors text-gray-600",
            "px-4 transition-colors text-gray-600 py-2",
        ];
        
        for input in test_cases {
            let result = processor.process_with_fallback(input, false);
            
            // Count spaces
            let input_spaces = input.chars().filter(|&c| c == ' ').count();
            let result_spaces = result.chars().filter(|&c| c == ' ').count();
            
            assert!(
                result_spaces >= input_spaces,
                "Whitespace count mismatch for '{}'. Input has {} spaces, result has {} spaces. Result: '{}'",
                input, input_spaces, result_spaces, result
            );
        }
    }
}
