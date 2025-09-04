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
