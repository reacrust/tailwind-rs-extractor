#[test]
fn test_trace_import_string() {
    use tailwind_rs::TailwindBuilder;
    
    let mut builder = TailwindBuilder::default();
    
    // Test trace with the problematic import string
    let test_cases = vec![
        ("react/jsx-runtime", "react/jsx-runtime"),  // Should be unchanged
        ("react", "react"),                           // May be transformed (trace returns Owned)
        ("jsx-runtime", "jsx-runtime"),               // Should be unchanged
        ("p-4 bg-blue-500", "p-4 bg-blue-500"),      // May be transformed
        // Note: "@rescript/core" is incorrectly transformed to "@rescript" by trace()
        // This is a bug in tailwind-rs where it treats "/core" as a modifier
        ("@rescript/core", "@rescript"),              // Bug: incorrectly transformed
    ];
    
    for (input, expected) in &test_cases {
        match builder.trace(input, false) {
            Ok(result) => {
                // Convert Cow to String for comparison
                let result_str = result.as_ref();
                println!("trace(\"{}\") = \"{}\"", input, result_str);
                assert_eq!(result_str, *expected, 
                    "Unexpected transformation: \"{}\" -> \"{}\" (expected \"{}\")", 
                    input, result_str, expected);
            }
            Err(e) => {
                panic!("trace(\"{}\") failed with error: {:?}", input, e);
            }
        }
    }
}