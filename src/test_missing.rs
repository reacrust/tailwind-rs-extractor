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

    #[test]
    fn test_arbitrary_values_with_decimals() {
        let source = r#"
// Test arbitrary values with decimal points
const test1 = "gap-[0.25rem] gap-[1.5rem]";
const test2 = "leading-[162.5%]";
const test3 = condition ? "gap-[0.25rem]" : "gap-[1.5rem]";
const test4 = ["gap-[0.25rem]", "leading-[162.5%]"].join(" ");
const test5 = `${baseClass} gap-[0.25rem] leading-[162.5%]`;
        "#;

        let config = TransformConfig::default();
        let (_, metadata) = transform_source(source, config).unwrap();

        // Check that decimal arbitrary values are extracted correctly
        let expected_classes = vec![
            "gap-[0.25rem]",
            "gap-[1.5rem]",
            "leading-[162.5%]",
        ];

        for class in expected_classes {
            assert!(
                metadata.classes.contains(&class.to_string()),
                "Missing arbitrary value class: {}. Extracted classes: {:?}",
                class,
                metadata.classes
            );
        }
    }
}