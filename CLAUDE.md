# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Tailwind Extractor is a high-performance Webpack/RSpack plugin that extracts and processes Tailwind CSS classes from JavaScript/TypeScript code using a Rust-based CLI. The project consists of:

- **NPM Package**: JavaScript plugin for Webpack/RSpack that coordinates extraction and CSS generation
- **Rust CLI Binary**: Pre-compiled binary (`bins/x86_64-linux/tailwind-extractor-cli`) that performs the actual AST transformation and class extraction
- **Rust Crate**: The source code for the CLI binary is in this same repository (Rust crate)
- **Unified Plugin Architecture**: Single plugin class that automatically registers both loader and plugin components

## Common Development Commands

```bash
# Run all tests
npm test

# Run specific test file
npm test -- plugin.test.js

# Run tests in watch mode
npm test:watch

# Run tests with coverage
npm test:coverage

# Debug tests with verbose output
npm test:debug

# Run a single test by name
npm test -- -t "should extract and generate CSS"

# Rebuild static binaries for all platforms (in progress)
nix run .#build-static-binaries
```

## Architecture

### Plugin Architecture

The plugin (`index.js`) follows a two-phase approach:

1. **Transformation Phase** (Loader - `lib/loader.js`):
   - Processes each JS/JSX/TS file through the Rust CLI
   - Extracts Tailwind classes and writes metadata JSON files
   - Uses unique filenames per source to avoid concurrent write conflicts
   - Transforms certain classes (e.g., `font-bold` → `font-[700]`)

2. **Generation Phase** (Plugin):
   - Merges all metadata files from temp directory
   - Calls CLI with `generate` command to create CSS
   - Outputs both CSS file (with hash) and manifest file (stable name)
   - Cleans up temp directory unless `keepTempDir: true`

### Key Design Decisions

1. **Unified Plugin**: Users only need to add one plugin - it auto-registers the loader to prevent misconfiguration

2. **Temp Directory Management**: Uses `fs.mkdtempSync()` for unique temp directories per build, avoiding conflicts in parallel builds

3. **Metadata Aggregation**: Each source file gets its own metadata file, merged before CSS generation to handle concurrent processing

4. **Binary Selection**: Automatically selects platform-specific binary from `bins/` directory based on OS/arch unless explicitly overridden

### Class Extraction Behavior

The Rust CLI extractor:
- **Extracts**: Static string literals containing Tailwind classes
- **Does NOT extract**: Classes within template literal expressions (e.g., `` `${isActive ? 'bg-blue' : 'bg-red'}` ``)
- **Supports**: Ternary operators with static strings, object literals with class strings
- **Transforms**: Some classes to normalized forms (e.g., `font-bold` → `font-[700]`, `gap-7` → `gap-[1.75rem]`)

## Testing Strategy

Tests are organized by concern:

- `plugin.test.js`: Core plugin functionality, CSS generation, configuration options
- `runtime.test.js`: Verifies transformed JavaScript executes correctly with React
- `static-runtime.test.js`: Tests static Tailwind classes render properly
- `transform.test.js`: Tests CLI transformation preserves JavaScript semantics

### Important Test Notes

- Tests use real Tailwind classes, not `unique-*` prefixes (those won't be extracted)
- The CLI only extracts classes from static strings, not dynamic expressions
- Some Tailwind classes are transformed during extraction (e.g., font weights)
- Tests create temp directories using `tmp.dirSync()` and clean up after

## Binary Management

The package includes pre-built binaries in `bins/` directory for easier distribution:
- Currently only `x86_64-linux` is available
- Binary is selected automatically based on platform/architecture
- Override with `tailwindExtractorPath` option if needed

The Rust source code for the CLI binary is included in this repository. To rebuild the static binaries:
- Run `nix run .#build-static-binaries` (currently in progress)
- GitHub Actions will automatically rebuild binaries on every release (planned)

## Manifest File

The plugin generates a `tailwind.manifest.json` containing:
- All extracted Tailwind classes
- Source files processed
- Processing timestamp and statistics
- Intended for SSR engines to avoid regenerating existing classes

The manifest has a stable, predictable filename (no hash) for easy consumption by other tools.