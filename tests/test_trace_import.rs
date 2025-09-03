#[test]
fn test_trace_import_string() {
    use tailwind_rs::TailwindBuilder;
    
    let mut builder = TailwindBuilder::default();
    
    // Test trace with import strings - they should NOT be transformed
    let test_cases = vec![
        ("react/jsx-runtime", "react/jsx-runtime"),  // Should be unchanged
        ("react", "react"),                           // Should be unchanged  
        ("jsx-runtime", "jsx-runtime"),               // Should be unchanged
        ("p-4 bg-blue-500", "p-4 bg-blue-500"),      // May be transformed (valid Tailwind classes)
        ("@rescript/core", "@rescript/core"),         // Should be unchanged - it's an import path!
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