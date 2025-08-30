# Tailwind Extractor

A Rust-based tool for extracting Tailwind CSS classes from JavaScript/TypeScript source files and generating optimized CSS.

## Overview

The tailwind-extractor scans JavaScript/TypeScript files (including bundled code) for Tailwind CSS classes and generates the corresponding CSS. It integrates with build tools like RSpack/Webpack to provide automatic CSS generation during the build process.

## Architecture

### Class Name Transformation and Hydration Alignment

#### The Problem

In SSR (Server-Side Rendering) applications, there's a critical issue with Tailwind class name consistency:

1. **During SSR**: The server (via `V8DirectRenderer`) renders React components and uses `TailwindBuilder::trace()` to process class names. This `trace()` method normalizes/transforms classes even without obfuscation (e.g., `bg-white` → `bg-[#FFFFFFFF]`, `max-w-4xl` → `max-width-4xl`).

2. **In Client Bundles**: The JavaScript bundles contain the original, untransformed class names as written in the source code.

3. **During Hydration**: React compares the server-rendered HTML (with transformed classes) against what it would render on the client (with original classes), detects a mismatch, and triggers a full re-render causing visible layout shifts.

#### The Solution

The tailwind-extractor will be enhanced to perform the same class name transformations during build time:

1. **AST Traversal**: Already implemented - we parse JavaScript into an AST and traverse all string literals looking for Tailwind classes.

2. **Class Transformation**: Apply the same `TailwindBuilder::trace()` transformations that the SSR uses.

3. **AST Mutation**: Modify the string literals in-place within the AST.

4. **Code Generation**: Serialize the modified AST back to JavaScript and return it to the bundler.

This ensures that both server-rendered HTML and client-side JavaScript use identical, transformed class names, eliminating hydration mismatches.

### Shared Class Processing Logic

The class transformation logic needs to be identical between SSR and build-time processing. This will be achieved by extracting the transformation logic from `react_ssr::V8DirectRenderer` into this crate.

#### Four-Tier Fallback Strategy

The transformation logic handles mixed Tailwind and custom CSS classes using a smart fallback strategy:

1. **Tier 1 - Optimal Path**: Try processing the entire class string at once
   - Best for pure Tailwind classes
   - Enables group optimization/obfuscation

2. **Tier 2 - Custom First Pattern**: Process all except the first class
   - Handles patterns like `"my-component bg-blue-500 text-white"`
   - Preserves custom class at the beginning

3. **Tier 3 - Custom Last Pattern**: Process all except the last class
   - Handles patterns like `"bg-blue-500 text-white my-component"`
   - Preserves custom class at the end

4. **Tier 4 - Individual Processing**: Process each class separately
   - Fallback for complex mixed patterns
   - Custom classes pass through unchanged
   - Tailwind classes get transformed individually

#### Error Handling

- **Valid Tailwind classes**: Transformed/normalized by `trace()`
- **Custom/unknown classes**: Passed through unchanged
- **Invalid syntax**: Silently passed through without errors
- **Class order**: Always preserved regardless of processing tier

### Two-Tier CSS Generation System

The architecture supports two distinct sources of Tailwind classes:

1. **Static Classes** (from source code):
   - Extracted at build time by this tool
   - Transformed during extraction to match SSR output
   - Generated CSS written to static `.css` files
   - Bundled and cached efficiently

2. **Dynamic Classes** (from runtime data):
   - Come from databases, APIs, or user input
   - Processed by SSR using the same transformation logic
   - Generated CSS injected as `<style>` tags in the HTML
   - Cannot be pre-processed at build time

### Integration Points

#### RSpack/Webpack Plugin

The `rspack-plugin` directory contains a plugin that:
- Integrates with the build pipeline
- Intercepts JavaScript assets
- Runs the extractor on the code
- Returns both transformed JavaScript and generated CSS
- Injects CSS into the HTML via HtmlWebpackPlugin

#### CLI Tool

The `tailwind-extractor-cli` provides:
- Standalone extraction from files
- Pipe mode for processing streams
- Configuration options for preflight CSS
- Verbose logging for debugging

## Key Design Decisions

### Why Transform at Build Time?

1. **Consistency**: Ensures server and client use identical class names
2. **Performance**: Transformation happens once during build, not at runtime
3. **Compatibility**: Works with any SSR framework using the same `TailwindBuilder`
4. **Debugging**: Source maps can be updated to maintain debuggability

### Why Not Alternative Approaches?

**Runtime transformation on client**: Would add overhead to every page load and increase bundle size.

**Disable transformation in SSR**: The `trace()` method always normalizes classes (even without obfuscation) for consistency and optimization.

**Separate transformation step**: Integrating into the existing extraction process is simpler and more efficient.

## Usage

### With RSpack/Webpack

```javascript
const TailwindExtractorPlugin = require('tailwind-extractor/rspack-plugin');

module.exports = {
  plugins: [
    new TailwindExtractorPlugin({
      // Transforms class names to match SSR output
      transformClasses: true,
      // Other options...
    })
  ]
};
```

### CLI

```bash
# Extract and transform classes from built bundles
tailwind-extractor-cli extract --transform dist/bundle.js

# Process streaming input with transformation
cat dist/app.js | tailwind-extractor-cli pipe --transform
```

## Implementation Status

- ✅ AST parsing and traversal
- ✅ Tailwind class extraction
- ✅ CSS generation
- ⏳ Class name transformation (planned)
- ⏳ AST mutation and code generation (planned)
- ⏳ Shared transformation logic extraction (planned)