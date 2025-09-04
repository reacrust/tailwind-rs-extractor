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
}
