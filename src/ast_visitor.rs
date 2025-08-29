use swc_core::common::{SourceMap, Span, FileName, Globals, GLOBALS};
use swc_core::ecma::ast::*;
use swc_core::ecma::parser::{parse_file_as_module, EsSyntax, Syntax, TsSyntax};
use swc_core::ecma::visit::{Visit, VisitWith};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use crate::errors::{ExtractorError, Result};

/// Information about an extracted string
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtractedString {
    /// The actual string value (single class name after splitting)
    pub value: String,
    /// Source file path
    pub file_path: String,
    /// Line number in source file (1-indexed)
    pub line: usize,
    /// Column number in source file (0-indexed)
    pub column: usize,
}

/// Context for extracting strings from a file
pub struct ExtractionContext {
    pub file_path: String,
    pub source_map: Arc<SourceMap>,
}

/// Visitor that extracts string literals from JavaScript/TypeScript AST
pub struct StringLiteralExtractor {
    /// Extracted strings with their source locations
    pub strings: Vec<ExtractedString>,
    /// Context for the current file being processed
    context: ExtractionContext,
}

impl StringLiteralExtractor {
    pub fn new(context: ExtractionContext) -> Self {
        Self {
            strings: Vec::new(),
            context,
        }
    }

    /// Extract a string value and record its location
    fn extract_string(&mut self, value: &str, span: Span) {
        // Split the string on whitespace to extract individual class names
        for class_name in value.split_whitespace() {
            // Skip empty strings
            if class_name.is_empty() {
                continue;
            }

            // Get source location from span
            let loc = self.context.source_map.lookup_char_pos(span.lo);
            
            self.strings.push(ExtractedString {
                value: class_name.to_string(),
                file_path: self.context.file_path.clone(),
                line: loc.line,
                column: loc.col_display,
            });
        }
    }
}

impl Visit for StringLiteralExtractor {
    /// Visit string literals
    fn visit_str(&mut self, node: &Str) {
        self.extract_string(&node.value, node.span);
    }

    /// Visit template literals (backtick strings)
    fn visit_tpl(&mut self, node: &Tpl) {
        // Extract from template quasi elements (the string parts)
        for quasi in &node.quasis {
            // Handle both cooked and raw strings
            if let Some(cooked) = &quasi.cooked {
                self.extract_string(&cooked, quasi.span);
            } else {
                self.extract_string(&quasi.raw, quasi.span);
            }
        }
        
        // Continue visiting expressions within the template
        node.visit_children_with(self);
    }

    /// Visit JSX attributes
    fn visit_jsx_attr(&mut self, node: &JSXAttr) {
        // Extract from JSX attribute values
        if let Some(value) = &node.value {
            match value {
                JSXAttrValue::Lit(lit) => {
                    if let Lit::Str(str_lit) = lit {
                        self.extract_string(&str_lit.value, str_lit.span);
                    }
                }
                JSXAttrValue::JSXExprContainer(expr_container) => {
                    // Visit the expression container to find strings
                    expr_container.visit_children_with(self);
                }
                _ => {
                    // Continue visiting other types
                    value.visit_children_with(self);
                }
            }
        }
        
        // Visit children
        node.visit_children_with(self);
    }

    /// Visit JSX text nodes
    fn visit_jsx_text(&mut self, node: &JSXText) {
        // Extract text content from JSX
        let text = node.value.trim();
        if !text.is_empty() {
            self.extract_string(text, node.span);
        }
    }

    /// Visit object literal properties (for className objects)
    fn visit_prop(&mut self, node: &Prop) {
        match node {
            Prop::KeyValue(kv) => {
                // Extract string keys
                if let PropName::Str(str_key) = &kv.key {
                    self.extract_string(&str_key.value, str_key.span);
                }
                // Continue visiting the value
                kv.value.visit_children_with(self);
            }
            _ => {
                node.visit_children_with(self);
            }
        }
    }
}

/// Parse a JavaScript/TypeScript file and extract all string literals
pub fn extract_strings_from_file(file_path: &Path) -> Result<Vec<ExtractedString>> {
    // Read file content with early termination for empty files
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| ExtractorError::ParseError {
            path: file_path.display().to_string(),
            message: format!("Failed to read file: {}", e),
        })?;
    
    // Early termination for empty or very small files
    if content.len() < 10 {
        return Ok(Vec::new());
    }

    // Create source map
    let source_map = Arc::new(SourceMap::default());
    let source_file = source_map.new_source_file(
        FileName::Real(file_path.to_path_buf()).into(),
        content,
    );

    // Determine syntax based on file extension
    let syntax = if file_path.extension().and_then(|s| s.to_str()) == Some("ts") 
        || file_path.extension().and_then(|s| s.to_str()) == Some("tsx") {
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: false,
        })
    } else {
        Syntax::Es(EsSyntax {
            jsx: true,
            decorators: false,
            decorators_before_export: false,
            export_default_from: false,
            import_attributes: false,
            allow_super_outside_method: false,
            allow_return_outside_function: false,
            auto_accessors: false,
            explicit_resource_management: false,
            fn_bind: false,
        })
    };

    // Parse the module
    let module = GLOBALS.set(&Globals::new(), || {
        parse_file_as_module(
            &source_file,
            syntax,
            EsVersion::latest(),
            None,
            &mut vec![],
        )
        .map_err(|e| ExtractorError::ParseError {
            path: file_path.display().to_string(),
            message: format!("Failed to parse JavaScript/TypeScript: {:?}", e),
        })
    })?;

    // Create visitor and extract strings
    let mut visitor = StringLiteralExtractor::new(ExtractionContext {
        file_path: file_path.display().to_string(),
        source_map,
    });
    
    module.visit_with(&mut visitor);

    Ok(visitor.strings)
}

/// Parse JavaScript/TypeScript content and extract all string literals
pub fn extract_strings_from_content(content: &str, source_name: &str) -> Result<Vec<ExtractedString>> {
    
    // Early termination for empty or very small content
    if content.len() < 10 {
        return Ok(Vec::new());
    }

    // Create source map
    let source_map = Arc::new(SourceMap::default());
    let source_file = source_map.new_source_file(
        FileName::Custom(source_name.to_string()).into(),
        content.to_string(),
    );

    // Use JavaScript syntax with JSX support (most permissive for pipe mode)
    let syntax = Syntax::Es(EsSyntax {
        jsx: true,
        decorators: false,
        decorators_before_export: false,
        export_default_from: false,
        import_attributes: false,
        allow_super_outside_method: false,
        allow_return_outside_function: false,
        auto_accessors: false,
        explicit_resource_management: false,
        fn_bind: false,
    });

    // Parse the module
    let module = GLOBALS.set(&Globals::new(), || {
        parse_file_as_module(
            &source_file,
            syntax,
            EsVersion::latest(),
            None,
            &mut vec![],
        )
        .map_err(|e| ExtractorError::ParseError {
            path: source_name.to_string(),
            message: format!("Failed to parse JavaScript/TypeScript: {:?}", e),
        })
    })?;

    // Create visitor and extract strings
    let mut visitor = StringLiteralExtractor::new(ExtractionContext {
        file_path: source_name.to_string(),
        source_map,
    });
    
    module.visit_with(&mut visitor);

    Ok(visitor.strings)
}

/// Extract unique class names from multiple files
pub fn extract_unique_classes(files: &[std::path::PathBuf]) -> Result<HashSet<String>> {
    let mut unique_classes = HashSet::new();

    for file_path in files {
        let extracted = extract_strings_from_file(file_path)?;
        for extracted_string in extracted {
            unique_classes.insert(extracted_string.value);
        }
    }

    Ok(unique_classes)
}

/// Process files in parallel and extract strings
pub fn extract_strings_parallel(
    files: &[std::path::PathBuf],
    jobs: Option<usize>,
) -> Result<Vec<ExtractedString>> {
    use rayon::prelude::*;
    
    // Configure thread pool only if specified and not already initialized
    if let Some(num_jobs) = jobs {
        // Try to build the global thread pool, but ignore if already initialized
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(num_jobs)
            .build_global();
    }

    // Process files in parallel
    let results: std::result::Result<Vec<_>, _> = files
        .par_iter()
        .map(|file_path| extract_strings_from_file(file_path))
        .collect();

    // Flatten results
    let mut all_strings = Vec::new();
    for file_results in results? {
        all_strings.extend(file_results);
    }

    Ok(all_strings)
}