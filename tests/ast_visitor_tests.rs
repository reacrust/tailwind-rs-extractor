use std::collections::HashSet;
use std::path::PathBuf;
use tailwind_extractor::ast_visitor::{extract_strings_from_file, extract_unique_classes, ExtractedString};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_extract_simple_strings() {
    let path = fixture_path("simple.js");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    // Collect all extracted values
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    // Check for expected Tailwind classes
    assert!(values.contains("bg-blue-500"));
    assert!(values.contains("text-white"));
    assert!(values.contains("p-4"));
    assert!(values.contains("hover:bg-blue-600"));
    assert!(values.contains("flex"));
    assert!(values.contains("items-center"));
    assert!(values.contains("justify-between"));
    assert!(values.contains("max-w-7xl"));
    assert!(values.contains("mx-auto"));
    assert!(values.contains("text-3xl"));
    assert!(values.contains("font-semibold"));
    assert!(values.contains("rounded-lg"));
    assert!(values.contains("shadow-md"));
    assert!(values.contains("border"));
    assert!(values.contains("border-gray-200"));
}

#[test]
fn test_extract_from_template_literals() {
    let path = fixture_path("simple.js");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    // Check template literal content is extracted
    assert!(values.contains("flex"));
    assert!(values.contains("items-center"));
    assert!(values.contains("justify-between"));
    assert!(values.contains("font-bold")); // From template with expression
}

#[test]
fn test_extract_from_react_component() {
    let path = fixture_path("react_component.mjs");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    // Check base button classes
    assert!(values.contains("px-4"));
    assert!(values.contains("py-2"));
    assert!(values.contains("font-semibold"));
    assert!(values.contains("rounded-lg"));
    assert!(values.contains("transition-colors"));
    
    // Check variant classes
    assert!(values.contains("bg-blue-500"));
    assert!(values.contains("hover:bg-blue-600"));
    assert!(values.contains("bg-gray-200"));
    assert!(values.contains("hover:bg-gray-300"));
    
    // Check card classes
    assert!(values.contains("bg-white"));
    assert!(values.contains("rounded-xl"));
    assert!(values.contains("shadow-lg"));
    assert!(values.contains("p-6"));
    assert!(values.contains("text-2xl"));
    assert!(values.contains("font-bold"));
    assert!(values.contains("mb-4"));
    assert!(values.contains("space-y-2"));
    
    // Check status classes
    assert!(values.contains("bg-green-100"));
    assert!(values.contains("text-green-800"));
    assert!(values.contains("bg-yellow-100"));
    assert!(values.contains("text-yellow-800"));
}

#[test]
fn test_extract_from_rescript_output() {
    let path = fixture_path("rescript_output.res.mjs");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    // Check Header component classes
    assert!(values.contains("flex"));
    assert!(values.contains("items-center"));
    assert!(values.contains("justify-between"));
    assert!(values.contains("px-6"));
    assert!(values.contains("py-4"));
    assert!(values.contains("bg-white"));
    assert!(values.contains("shadow-sm"));
    assert!(values.contains("text-3xl"));
    assert!(values.contains("font-bold"));
    assert!(values.contains("text-gray-900"));
    assert!(values.contains("mt-1"));
    assert!(values.contains("text-sm"));
    assert!(values.contains("text-gray-500"));
    
    // Check Navigation classes
    assert!(values.contains("space-x-4"));
    assert!(values.contains("px-3"));
    assert!(values.contains("py-2"));
    assert!(values.contains("font-medium"));
    assert!(values.contains("text-blue-600"));
    assert!(values.contains("bg-blue-50"));
    assert!(values.contains("rounded-md"));
    assert!(values.contains("text-gray-700"));
    assert!(values.contains("hover:text-gray-900"));
    assert!(values.contains("hover:bg-gray-50"));
    
    // Check Layout classes
    assert!(values.contains("min-h-screen"));
    assert!(values.contains("bg-gray-50"));
    assert!(values.contains("flex-1"));
    assert!(values.contains("px-8"));
    assert!(values.contains("max-w-7xl"));
    assert!(values.contains("mx-auto"));
    assert!(values.contains("px-4"));
    assert!(values.contains("sm:px-6"));
    assert!(values.contains("lg:px-8"));
    assert!(values.contains("w-64"));
    assert!(values.contains("shadow-md"));
}

#[test]
fn test_source_location_tracking() {
    let path = fixture_path("simple.js");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    // Find a specific class to check its location
    let bg_blue = extracted.iter().find(|e| e.value == "bg-blue-500").unwrap();
    assert_eq!(bg_blue.line, 2); // Line 2 in the file
    assert!(bg_blue.file_path.ends_with("simple.js"));
    
    // Check that column is tracked
    assert!(bg_blue.column > 0);
}

#[test]
fn test_whitespace_splitting() {
    let path = fixture_path("simple.js");
    let extracted = extract_strings_from_file(&path).unwrap();
    
    // The string "bg-blue-500 text-white p-4" should be split into 3 separate entries
    let from_first_line: Vec<&ExtractedString> = extracted
        .iter()
        .filter(|e| e.line == 2)
        .collect();
    
    // Should have at least the 3 classes from the first className
    assert!(from_first_line.len() >= 3);
    
    let values: HashSet<&str> = from_first_line.iter().map(|e| e.value.as_str()).collect();
    assert!(values.contains("bg-blue-500"));
    assert!(values.contains("text-white"));
    assert!(values.contains("p-4"));
}

#[test]
fn test_extract_unique_classes() {
    let files = vec![
        fixture_path("simple.js"),
        fixture_path("react_component.mjs"),
    ];
    
    let unique_classes = extract_unique_classes(&files).unwrap();
    
    // Should have unique classes from both files
    assert!(unique_classes.contains("bg-blue-500"));
    assert!(unique_classes.contains("text-white"));
    assert!(unique_classes.contains("hover:bg-blue-600"));
    assert!(unique_classes.contains("transition-colors"));
    assert!(unique_classes.contains("rounded-xl"));
    
    // Check that duplicates are removed (bg-white appears in both files)
    let bg_white_count = unique_classes.iter().filter(|&c| c == "bg-white").count();
    assert_eq!(bg_white_count, 1);
}

#[test]
fn test_empty_strings_ignored() {
    // Create a temporary test file with empty strings
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test.js");
    std::fs::write(&test_file, r#"
        const empty = "";
        const spaces = "   ";
        const valid = "bg-red-500";
        const mixed = "  text-blue-500  ";
    "#).unwrap();
    
    let extracted = extract_strings_from_file(&test_file).unwrap();
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    // Should only have the valid classes, not empty strings
    assert!(values.contains("bg-red-500"));
    assert!(values.contains("text-blue-500"));
    assert_eq!(values.len(), 2);
}

#[test]
fn test_jsx_attribute_extraction() {
    // Create a test file with JSX
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsx");
    std::fs::write(&test_file, r#"
        const Component = () => (
            <div className="container mx-auto">
                <button className="btn btn-primary" aria-label="Submit">
                    Click me
                </button>
            </div>
        );
    "#).unwrap();
    
    let extracted = extract_strings_from_file(&test_file).unwrap();
    let values: HashSet<String> = extracted.iter().map(|e| e.value.clone()).collect();
    
    assert!(values.contains("container"));
    assert!(values.contains("mx-auto"));
    assert!(values.contains("btn"));
    assert!(values.contains("btn-primary"));
    assert!(values.contains("Submit")); // aria-label is also extracted
}