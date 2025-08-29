# Tailwind Extractor RSpack Plugin

RSpack plugin for extracting Tailwind CSS classes from compiled ReScript/JavaScript files using the `tailwind-extractor-cli` tool.

## Installation

```bash
# If published to npm (future)
npm install @capitalmadrid/tailwind-extractor-plugin --save-dev

# For local development, add to package.json dependencies:
# "file:./crates/tailwind-extractor/rspack-plugin"
```

## Usage

### Basic Configuration

```javascript
// rspack.config.js
const TailwindExtractorPlugin = require('@capitalmadrid/tailwind-extractor-plugin');

module.exports = {
  plugins: [
    new TailwindExtractorPlugin({
      input: ['src/**/*.res.mjs', 'src/**/*.js'],
      outputCss: 'dist/tailwind.css',
      outputManifest: 'dist/tailwind-manifest.json',
    }),
  ],
};
```

### Advanced Configuration

```javascript
// rspack.config.js
const TailwindExtractorPlugin = require('@capitalmadrid/tailwind-extractor-plugin');

module.exports = {
  plugins: [
    new TailwindExtractorPlugin({
      input: ['src/**/*.res.mjs', 'src/**/*.js'],
      outputCss: 'dist/tailwind.css',
      outputManifest: 'dist/tailwind-manifest.json',
      
      // Optional settings
      cliPath: './custom/path/to/tailwind-extractor-cli', // Auto-detected if not provided
      config: './tailwind-extractor.yaml', // YAML configuration file
      obfuscate: process.env.NODE_ENV === 'production', // Obfuscate class names in production
      minify: process.env.NODE_ENV === 'production', // Minify CSS in production
      verbose: process.env.NODE_ENV === 'development', // Verbose logging
      jobs: 4, // Number of parallel threads (defaults to CPU cores)
      exclude: ['node_modules/**', 'test/**'], // Patterns to exclude
      dryRun: false, // Don't write files (for testing)
    }),
  ],
};
```

## Options

### Required Options

- **`input`** (`string[]`): Array of glob patterns for input files to scan for Tailwind classes
- **`outputCss`** (`string`): Path where the generated CSS file will be written
- **`outputManifest`** (`string`): Path where the JSON manifest will be written

### Optional Options

- **`cliPath`** (`string`): Path to the `tailwind-extractor-cli` binary. Auto-detected if not provided
- **`config`** (`string`): Path to YAML configuration file
- **`obfuscate`** (`boolean`, default: `false`): Enable obfuscation of Tailwind class names
- **`minify`** (`boolean`, default: `false`): Enable minification of the output CSS
- **`verbose`** (`boolean`, default: `false`): Enable verbose logging output
- **`jobs`** (`number`): Number of parallel threads to use (defaults to number of CPU cores)
- **`exclude`** (`string[]`, default: `[]`): Array of glob patterns to exclude from scanning
- **`dryRun`** (`boolean`, default: `false`): Perform extraction but don't write output files

## How It Works

1. The plugin hooks into RSpack's `beforeCompile` phase
2. Executes the `tailwind-extractor-cli` tool with the specified options
3. The CLI tool scans the input files using AST parsing to find Tailwind class names
4. Generates optimized CSS containing only the used Tailwind utilities
5. Creates a JSON manifest with metadata and optional obfuscation mappings
6. RSpack continues with the build process, and you can import the generated CSS

## Integration with ReScript SSR

This plugin is designed to work alongside the `react_ssr` crate's Tailwind integration:

- **Build-time extraction** (this plugin): Generates static CSS for production builds
- **Runtime processing** (`react_ssr`): Handles dynamic class generation during server-side rendering

Both approaches complement each other for a complete Tailwind CSS solution.

## Error Handling

The plugin will:
- Validate all required options during construction
- Log helpful error messages if the CLI tool fails
- Stop the build process if extraction fails (preventing broken CSS output)
- Auto-detect the CLI binary location in common scenarios

## Platform Support

- **Node.js**: 16.0.0 or higher
- **Operating Systems**: Windows, macOS, Linux (matches CLI tool support)
- **RSpack**: 1.0.0 or higher

## Development

The plugin is located in the same repository as the CLI tool:
```
/crates/tailwind-extractor/rspack-plugin/
├── tailwind-extractor-plugin.js
├── package.json
└── README.md
```

For local development, you can reference it directly:
```json
{
  "dependencies": {
    "tailwind-extractor-plugin": "file:./crates/tailwind-extractor/rspack-plugin"
  }
}
```