# CLAUDE.md - tailwind-extractor crate

## Critical Architecture: AST Mutation for Hydration Alignment

### The Problem
`TailwindBuilder::trace()` transforms class names during SSR (e.g., `bg-white` → `bg-[#FFFFFFFF]`), but client bundles retain original classes, causing React hydration mismatches and layout shifts.

### The Solution
**Build-time AST mutation**: Transform class names in JavaScript during build to match SSR output.

1. Parse JS/TS into AST (already done)
2. Traverse all string literals (already done)
3. Apply `trace()` transformations (TODO)
4. Mutate AST in-place (TODO)
5. Serialize back to JavaScript (TODO)
6. Return to bundler (TODO)

### Shared Transformation Logic

Extract from `react_ssr::V8DirectRenderer::process_with_fallback`:

```rust
// 4-tier fallback strategy for mixed Tailwind/custom classes:
// 1. Try entire string (optimal for pure Tailwind)
// 2. Try without first class (custom prefix pattern)
// 3. Try without last class (custom suffix pattern)  
// 4. Process individually (complex mixing fallback)
```

This EXACT logic must be used in both:
- **SSR**: `V8DirectRenderer` during rendering
- **Build**: `tailwind-extractor` during bundling

### Key Implementation Points

1. **ALWAYS transform**: Even in development mode, `trace(obfuscate=false)` still normalizes
2. **Preserve custom classes**: Unknown classes pass through unchanged
3. **Maintain order**: Class order must be preserved
4. **Handle errors gracefully**: Invalid classes silently pass through

### Two-Tier CSS System

1. **Static classes** (source code) → Build-time extraction → CSS files
2. **Dynamic classes** (database/runtime) → SSR processing → `<style>` tags

Both MUST use identical transformation logic.

## Development Guidelines

- Test with `ssr_demo` example to verify hydration works
- Use `SSR_DEMO_DEV=1` environment variable to allow eval in CSP
- Check console for hydration mismatch errors
- Verify both server HTML and client expectations match