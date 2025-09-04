#[cfg(test)]
mod missing_classes_tests {
    use crate::ast_transformer::{transform_source, TransformConfig};

    #[test]
    fn test_missing_classes_extraction() {
        let source = r#"
// Test the 7 missing classes
const test1 = condition ? "hover:bg-gray-100" : "text-gray-600";
const test2 = "flex " + "justify-between";
const test3 = ["lg:flex-row", "lg:w-80"].join(" ");
const test4 = isActive && "flex-shrink-0";
const test5 = isDark ? "hover:bg-blue-600" : "hover:bg-gray-600";
        "#;

        let config = TransformConfig::default();
        let (_, metadata) = transform_source(source, config).unwrap();

        // Check all 7 missing classes are extracted
        let expected_classes = vec![
            "hover:bg-gray-100",
            "hover:bg-blue-600",
            "hover:bg-gray-600",
            "justify-between",
            "lg:flex-row",
            "lg:w-80",
            "flex-shrink-0",
        ];

        for class in expected_classes {
            assert!(
                metadata.classes.contains(&class.to_string()),
                "Missing class: {}",
                class
            );
        }
    }
}