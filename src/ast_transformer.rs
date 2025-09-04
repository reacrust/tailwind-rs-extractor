//! AST-based JavaScript/TypeScript transformation for Tailwind CSS class processing
//!
//! This module provides the core AST transformation engine that:
//! - Parses JavaScript/TypeScript using SWC
//! - Visits string literals in relevant contexts
//! - Transforms class strings using TailwindClassProcessor
//! - Returns transformed code and class metadata

use anyhow::{Context, Result};
use indexmap::IndexSet;
use swc_core::{
    common::{
        comments::SingleThreadedComments, sync::Lrc, FileName, Globals, SourceMap,
        GLOBALS,
    },
    ecma::{
        ast::*,
        codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter},
        parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax},
        visit::{noop_visit_mut_type, VisitMut, VisitMutWith},
    },
};

use crate::TailwindClassProcessor;
use tailwind_rs::TailwindBuilder;

/// Metadata collected during AST transformation
#[derive(Debug, Clone)]
pub struct TransformMetadata {
    /// Deduplicated list of all classes discovered
    pub classes: Vec<String>,
    /// Count of classes before deduplication
    pub original_count: usize,
}

/// Configuration for AST transformation
#[derive(Debug, Clone)]
pub struct TransformConfig {
    /// Whether to obfuscate Tailwind classes
    pub obfuscate: bool,
    /// Whether to preserve source maps (if applicable)
    pub source_maps: bool,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            obfuscate: false,
            source_maps: false,
        }
    }
}

/// AST visitor that transforms Tailwind classes in string literals
struct TailwindTransformer {
    /// Tailwind builder for class processing
    tailwind: TailwindBuilder,
    /// Configuration settings
    config: TransformConfig,
    /// Collected classes (deduplicated)
    classes: IndexSet<String>,
    /// Count of all classes before deduplication
    total_count: usize,
}

impl TailwindTransformer {
    fn new(config: TransformConfig) -> Result<Self> {
        let tailwind = TailwindBuilder::default();
        Ok(Self {
            tailwind,
            config,
            classes: IndexSet::new(),
            total_count: 0,
        })
    }

    /// Process a string literal and transform its classes
    fn process_string(&mut self, value: &str) -> String {
        // Always use trace() to process the string
        let processed = match self.tailwind.trace(value, self.config.obfuscate) {
            Ok(result) => result.into_owned(),
            Err(_) => value.to_string(), // Fallback to original on error
        };

        // Extract individual classes for metadata
        self.extract_classes(value);

        processed
    }

    /// Extract individual classes from a string for metadata collection
    fn extract_classes(&mut self, value: &str) {
        // Split on whitespace and collect non-empty tokens
        for class in value.split_whitespace() {
            if !class.is_empty() {
                self.classes.insert(class.to_string());
                self.total_count += 1;
            }
        }
    }

    /// Check if we should process this string based on context
    fn should_process_string(&self) -> bool {
        // We process all string literals in relevant contexts
        // This could be refined based on parent node context if needed
        true
    }
}

impl TailwindClassProcessor for TailwindTransformer {
    fn tailwind_builder(&mut self) -> &mut TailwindBuilder {
        &mut self.tailwind
    }
}

impl VisitMut for TailwindTransformer {
    noop_visit_mut_type!();

    /// Visit string literals and transform them
    fn visit_mut_str(&mut self, node: &mut Str) {
        if self.should_process_string() {
            let processed = self.process_string(&node.value);
            node.value = processed.into();
            node.raw = None; // Clear raw to use processed value
        }
    }

    /// Visit if statements to handle JSX assignments
    fn visit_mut_if_stmt(&mut self, node: &mut IfStmt) {
        // Visit the test condition
        node.test.visit_mut_with(self);
        
        // Visit consequent (then) block
        node.cons.visit_mut_with(self);
        
        // Visit alternate (else) block if present
        if let Some(alt) = &mut node.alt {
            alt.visit_mut_with(self);
        }
    }

    /// Visit JSX attributes (className, class)
    fn visit_mut_jsx_attr(&mut self, node: &mut JSXAttr) {
        // Check if this is a className or class attribute
        if let JSXAttrName::Ident(ident) = &node.name {
            if matches!(ident.sym.as_ref(), "className" | "class") {
                // Visit the value specifically for class attributes
                if let Some(value) = &mut node.value {
                    value.visit_mut_children_with(self);
                    return;
                }
            }
        }
        node.visit_mut_children_with(self);
    }

    /// Visit template literals (but not their interpolations)
    fn visit_mut_tpl(&mut self, node: &mut Tpl) {
        // Process only the string parts, not expressions (interpolations)
        for quasi in &mut node.quasis {
            if let Some(cooked) = &quasi.cooked {
                let cooked_str = cooked.to_string();
                let processed = self.process_string(&cooked_str);
                quasi.cooked = Some(processed.into());
                quasi.raw = quasi.cooked.clone().unwrap_or_default(); // Update raw to match
            }
        }
        // Don't visit expressions (interpolations)
    }

    /// Visit object literal properties
    fn visit_mut_prop(&mut self, node: &mut Prop) {
        match node {
            Prop::KeyValue(kv) => {
                // Process both key and value if they're strings
                if let PropName::Str(str_key) = &mut kv.key {
                    let processed = self.process_string(&str_key.value);
                    str_key.value = processed.into();
                    str_key.raw = None;
                }
                kv.value.visit_mut_with(self);
            }
            _ => node.visit_mut_children_with(self),
        }
    }

    /// Visit conditional (ternary) expressions
    fn visit_mut_cond_expr(&mut self, node: &mut CondExpr) {
        // Visit the test condition first
        node.test.visit_mut_with(self);
        
        // Visit consequent branch (true case)
        node.cons.visit_mut_with(self);
        
        // Visit alternate branch (false case)
        node.alt.visit_mut_with(self);
    }

    /// Visit binary expressions (string concatenation and logical operators)
    fn visit_mut_bin_expr(&mut self, node: &mut BinExpr) {
        // Handle string concatenation with + operator
        if matches!(node.op, BinaryOp::Add) {
            // Visit both operands for string extraction
            node.left.visit_mut_with(self);
            node.right.visit_mut_with(self);
        } 
        // Handle logical expressions (&& and ||)
        else if matches!(node.op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
            // Visit both operands - the right operand often contains class strings
            node.left.visit_mut_with(self);
            node.right.visit_mut_with(self);
        }
        else {
            // For other binary operations, still visit children
            node.visit_mut_children_with(self);
        }
    }

    /// Visit call expressions (for Array.join() and similar patterns)
    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        // Special handling for JSX function calls (JsxRuntime.jsx, JsxRuntime.jsxs)
        if let Callee::Expr(expr) = &mut node.callee {
            if let Expr::Member(member_expr) = &**expr {
                // Check for JsxRuntime.jsx or JsxRuntime.jsxs
                if let Expr::Ident(obj_ident) = &*member_expr.obj {
                    if obj_ident.sym.as_ref() == "JsxRuntime" {
                        if let MemberProp::Ident(method_ident) = &member_expr.prop {
                            if matches!(method_ident.sym.as_ref(), "jsx" | "jsxs") {
                                // This is a JSX element creation - process its props
                                self.visit_jsx_props(&mut node.args);
                                return;
                            }
                        }
                    }
                }
                
                // Check if this is a .join() call on an array
                if let MemberProp::Ident(ident) = &member_expr.prop {
                    if ident.sym.as_ref() == "join" {
                        // Visit the entire call expression's children to process array elements
                        node.visit_mut_children_with(self);
                        return;
                    }
                }
            }
        }
        
        // Visit all children to ensure we process strings in arguments
        node.visit_mut_children_with(self);
    }

    /// Visit array literals (for className arrays)
    fn visit_mut_array_lit(&mut self, node: &mut ArrayLit) {
        // Visit all array elements to extract classes
        for elem in &mut node.elems {
            if let Some(elem) = elem {
                elem.expr.visit_mut_with(self);
            }
        }
    }

    /// Visit assignment expressions to handle JSX assignments
    fn visit_mut_assign_expr(&mut self, node: &mut AssignExpr) {
        // Visit both left and right sides
        node.left.visit_mut_with(self);
        node.right.visit_mut_with(self);
    }
}

impl TailwindTransformer {
    /// Process JSX props to extract className values
    fn visit_jsx_props(&mut self, args: &mut Vec<ExprOrSpread>) {
        // JSX calls typically have two arguments: element name and props object
        if args.len() >= 2 {
            // The second argument is the props object
            let ExprOrSpread { expr, .. } = &mut args[1];
            // Visit the props object to extract className
            expr.visit_mut_with(self);
        }
    }
}

/// Transform JavaScript/TypeScript source code, processing Tailwind classes
pub fn transform_source(
    source: &str,
    config: TransformConfig,
) -> Result<(String, TransformMetadata)> {
    // Set up SWC components
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), source.to_string());

    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            ..Default::default()
        }),
        EsVersion::latest(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    // Parse the module
    let mut module = match parser.parse_module() {
        Ok(module) => module,
        Err(err) => {
            // Log error to stderr and return original source
            eprintln!("AST parsing error: {:?}", err);
            return Ok((
                source.to_string(),
                TransformMetadata {
                    classes: vec![],
                    original_count: 0,
                },
            ));
        }
    };

    // Apply transformation
    GLOBALS.set(&Globals::new(), || {
        // Create and apply our transformer
        let mut transformer = TailwindTransformer::new(config.clone())
            .context("Failed to create transformer")?;

        module.visit_mut_with(&mut transformer);

        // Generate the output code
        let mut buf = vec![];
        let mut emitter = Emitter {
            cfg: CodegenConfig::default(),
            cm: cm.clone(),
            comments: Some(&comments),
            wr: JsWriter::new(cm, "\n", &mut buf, None),
        };

        emitter.emit_module(&module).context("Failed to emit module")?;

        let code = String::from_utf8(buf).context("Failed to convert output to UTF-8")?;

        // Prepare metadata
        let metadata = TransformMetadata {
            classes: transformer.classes.into_iter().collect(),
            original_count: transformer.total_count,
        };

        Ok((code, metadata))
    })
}

/// Simple processor implementation for standalone usage
pub struct SimpleProcessor {
    tailwind: TailwindBuilder,
}

impl SimpleProcessor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tailwind: TailwindBuilder::default(),
        })
    }
}

impl TailwindClassProcessor for SimpleProcessor {
    fn tailwind_builder(&mut self) -> &mut TailwindBuilder {
        &mut self.tailwind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tailwind_rs::TailwindBuilder;

    fn trace_assert(string: &str, obfuscate: bool) -> String {
        let mut builder = TailwindBuilder::default();
        builder.trace(string, obfuscate).expect("should have traced it").to_string()
    }

    fn assert_does_not_transform_or_extract(source: &str) {
        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Should return original source on parse error
        assert_eq!(transformed, source, "transformed source!\n--- transformed:\n{}\n--- original:\n{}", transformed, source);
        assert_eq!(metadata.classes.len(), 0, "extracted_classes!\n--- source:\n{}\n---classes:\n{:?}", source, metadata.classes);
        assert_eq!(metadata.original_count, 0);
    }

    #[test]
    fn test_jsx_class_transformation() {
        let source = r#"
            const Button = () => (
                <button className="flex items-center hover:bg-blue-500">
                    Click me
                </button>
            );
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Should preserve the structure
        assert!(transformed.contains("className"));
        assert!(transformed.contains("Click me"));

        // Should extract the classes
        assert_eq!(metadata.original_count, 3);
        assert!(metadata.classes.contains(&"flex".to_string()));
        assert!(metadata.classes.contains(&"items-center".to_string()));
        assert!(metadata.classes.contains(&"hover:bg-blue-500".to_string()));
    }

    #[test]
    fn test_object_literal_keys() {
        let source = r#"
            const styles = {
                'text-center': true,
                'font-bold': false,
                container: 'mx-auto px-4'
            };
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Should preserve the structure
        assert!(transformed.contains("styles"));

        // Should extract classes from keys and values
        assert!(metadata.classes.contains(&"text-center".to_string()));
        assert!(metadata.classes.contains(&"font-bold".to_string()));
        assert!(metadata.classes.contains(&"mx-auto".to_string()));
        assert!(metadata.classes.contains(&"px-4".to_string()));
    }

    #[test]
    fn test_array_of_classes() {
        let source = r#"
            const classes = ['bg-white', 'shadow-lg', 'rounded-md', 'text-white'];
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        assert_eq!(metadata.classes.len(), 4);
        assert!(metadata.classes.contains(&"bg-white".to_string()));
        assert!(metadata.classes.contains(&"shadow-lg".to_string()));
        assert!(metadata.classes.contains(&"rounded-md".to_string()));
        assert!(metadata.classes.contains(&"text-white".to_string()));

        // transformed JS must contain transformed class-names 
        assert!(transformed.contains(&trace_assert("bg-white", false)), "{}", transformed);
        assert!(transformed.contains(&trace_assert("shadow-lg", false)), "{}", transformed);
        assert!(transformed.contains(&trace_assert("rounded-md", false)), "{}", transformed);
        assert!(transformed.contains(&trace_assert("text-white", false)), "{}", transformed);
    }

    #[test]
    fn test_template_literal_without_interpolation() {
        let source = r#"
            const className = `flex justify-between`;
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Classes are extracted
        assert!(metadata.classes.contains(&"flex".to_string()));
        assert!(metadata.classes.contains(&"justify-between".to_string()));

        // Order is preserved
        assert!(transformed.contains(&"flex justify-between".to_string()));

        assert!(transformed.contains(&trace_assert("flex justify-between", false)), "{}", transformed);
    }

    #[test]
    fn test_malformed_javascript() {
        let source = r#"cont x = "text-white" // syntax error"#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Should return original source on parse error
        assert_eq!(transformed, source);
        assert_eq!(metadata.classes.len(), 0);
        assert_eq!(metadata.original_count, 0);
    }

    #[test]
    fn test_does_not_break_imports() {
        assert_does_not_transform_or_extract(r#"import React from "react/client"\n"#);
    }

    #[test]
    fn test_deduplication() {
        let source = r#"
            const a = "flex flex";
            const b = "flex items-center";
        "#;

        let config = TransformConfig::default();
        let (_, metadata) = transform_source(source, config).unwrap();

        // Should have 4 original but only 2 unique
        assert_eq!(metadata.original_count, 4);
        assert_eq!(metadata.classes.len(), 2);
        assert!(metadata.classes.contains(&"flex".to_string()));
        assert!(metadata.classes.contains(&"items-center".to_string()));
    }

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

        for class in &expected_classes {
            assert!(
                metadata.classes.contains(&class.to_string()),
                "Missing class: {}",
                class
            );
        }
        
        // Also verify we got other classes
        assert!(metadata.classes.contains(&"text-gray-600".to_string()));
        assert!(metadata.classes.contains(&"flex".to_string()));
    }

    #[test]
    fn test_jsx_in_if_else_blocks() {
        let source = r#"
var tmp$1;
if (activeTab === "TailwindShowcase") {
  tmp$1 = null;
} else {
  tmp$1 = JsxRuntime.jsx("aside", {
    className: "lg:w-80 flex-shrink-0"
  });
}
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Classes should be extracted from JSX in else block
        assert!(metadata.classes.contains(&"lg:w-80".to_string()));
        assert!(metadata.classes.contains(&"flex-shrink-0".to_string()));
        
        // Transformed output should contain trace()
        assert!(transformed.contains(&trace_assert("lg:w-80 flex-shrink-0", false)));
    }

    #[test]
    fn test_jsx_in_ternary_expressions() {
        let source = r#"
const element = isActive 
  ? JsxRuntime.jsx("div", { className: "bg-blue-500 text-white" })
  : JsxRuntime.jsx("div", { className: "bg-gray-200 text-gray-600" });
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // All classes should be extracted
        assert!(metadata.classes.contains(&"bg-blue-500".to_string()));
        assert!(metadata.classes.contains(&"text-white".to_string()));
        assert!(metadata.classes.contains(&"bg-gray-200".to_string()));
        assert!(metadata.classes.contains(&"text-gray-600".to_string()));
        
        // Transformed output should contain trace() for both branches
        assert!(transformed.contains(&trace_assert("bg-blue-500 text-white", false)));
        assert!(transformed.contains(&trace_assert("bg-gray-200 text-gray-600", false)));
    }

    #[test]
    fn test_array_join_with_conditionals() {
        let source = r#"
const className = [
  "flex flex-col gap-6 p-6",
  activeTab === "TailwindShowcase" ? "" : "lg:flex-row"
].join(" ");
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // All classes from array elements should be extracted
        assert!(metadata.classes.contains(&"flex".to_string()));
        assert!(metadata.classes.contains(&"flex-col".to_string()));
        assert!(metadata.classes.contains(&"gap-6".to_string()));
        assert!(metadata.classes.contains(&"p-6".to_string()));
        assert!(metadata.classes.contains(&"lg:flex-row".to_string()));
        
        // Verify transformation
        assert!(transformed.contains(&trace_assert("flex flex-col gap-6 p-6", false)));
        assert!(transformed.contains(&trace_assert("lg:flex-row", false)));
    }

    #[test]
    fn test_nested_jsx_with_classname() {
        let source = r#"
JsxRuntime.jsxs("section", {
  children: [
    JsxRuntime.jsx("div", {
      children: content,
      className: "flex-1"
    }),
    JsxRuntime.jsx("aside", {
      children: JsxRuntime.jsx("div", {
        className: "bg-gray-50 p-6 rounded-lg"
      }),
      className: "lg:w-80 flex-shrink-0"
    })
  ],
  className: "flex flex-row gap-4"
});
        "#;

        let config = TransformConfig::default();
        let (_transformed, metadata) = transform_source(source, config).unwrap();

        // All nested classes should be extracted
        let expected_classes = vec![
            "flex-1",
            "bg-gray-50",
            "p-6",
            "rounded-lg",
            "lg:w-80",
            "flex-shrink-0",
            "flex",
            "flex-row",
            "gap-4",
        ];

        for class in &expected_classes {
            assert!(
                metadata.classes.contains(&class.to_string()),
                "Missing class from nested JSX: {}",
                class
            );
        }
    }

    #[test]
    fn test_jsx_runtime_calls() {
        let source = r#"
JsxRuntime.jsx("button", {
  className: "px-4 py-2 bg-indigo-500 hover:bg-indigo-600",
  onClick: handleClick
});
        "#;

        let config = TransformConfig::default();
        let (transformed, metadata) = transform_source(source, config).unwrap();

        // Classes should be extracted from JsxRuntime.jsx calls
        assert!(metadata.classes.contains(&"px-4".to_string()));
        assert!(metadata.classes.contains(&"py-2".to_string()));
        assert!(metadata.classes.contains(&"bg-indigo-500".to_string()));
        assert!(metadata.classes.contains(&"hover:bg-indigo-600".to_string()));
        
        // Verify transformation applied
        assert!(transformed.contains(&trace_assert("px-4 py-2 bg-indigo-500 hover:bg-indigo-600", false)));
    }
}
