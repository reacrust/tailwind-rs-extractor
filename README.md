# Tailwind Extractor

High-performance Tailwind CSS extraction and processing for RSpack/Webpack using a Rust-based CLI tool. This package provides a unified plugin that automatically configures both the loader and plugin components for seamless Tailwind CSS extraction during your build process.

## Features

- 🚀 **High Performance**: Rust-based CLI for blazing-fast CSS extraction
- 🔧 **Zero Configuration**: Single plugin handles both transformation and CSS generation
- 🎯 **Smart Detection**: Automatically processes JavaScript/TypeScript files containing Tailwind classes
- 🔒 **Production Ready**: Built-in support for class obfuscation and CSS minification
- 🏗️ **Framework Agnostic**: Works with RSpack, Webpack, and other bundlers
- 🧹 **Auto Cleanup**: Manages temporary files automatically

## Installation

```bash
npm install tailwind-extractor --save-dev
# or
yarn add -D tailwind-extractor
# or
pnpm add -D tailwind-extractor
```

## Quick Start

```javascript
// rspack.config.js or webpack.config.js
const TailwindExtractor = require('tailwind-extractor');

module.exports = {
  // ... your config
  plugins: [
    new TailwindExtractor({
      // All configuration is optional with sensible defaults
      css: {
        minify: true,           // Minify CSS output
        noPreflight: false,     // Include Tailwind reset styles
      }
    })
  ]
};
```

That's it! The plugin automatically:
- Registers a loader to transform your JavaScript/TypeScript files
- Extracts Tailwind classes during transformation
- Generates optimized CSS from the extracted classes
- Outputs a CSS file with a content hash (`tailwind.[hash].css`)

## Configuration Options

### Full Configuration Example

```javascript
new TailwindExtractor({
  // File matching options
  test: /\.(js|jsx|ts|tsx|mjs)$/,  // Which files to process
  exclude: /node_modules/,          // Files to exclude
  include: /src/,                    // Optional: only process specific directories

  // Transform options (for the loader)
  transform: {
    enabled: true,                   // Enable/disable transformation
    obfuscate: false,               // Obfuscate class names in production
  },

  // CSS generation options
  css: {
    noPreflight: false,             // Set to true to disable Tailwind reset styles
    minify: true,                   // Minify the generated CSS
  },

  // Debug options
  debug: false,                     // Enable debug logging
  keepTempDir: false,              // Keep temp directory for debugging

  // Output options
  manifestFilename: 'tailwind.manifest.json', // Manifest file name (set to false to disable)

  // Advanced options
  tailwindExtractorPath: 'tailwind-extractor-cli', // Path to CLI binary
})
```

### Option Details

#### File Matching Options

- **`test`** (RegExp): Pattern to match files for processing. Default: `/\.(js|jsx|ts|tsx|mjs)$/`
- **`exclude`** (RegExp): Pattern for files to exclude. Default: `/node_modules/`
- **`include`** (RegExp, optional): Pattern to limit processing to specific directories

#### Transform Options

- **`transform.enabled`** (boolean): Enable/disable the transformation. Default: `true`
- **`transform.obfuscate`** (boolean): Obfuscate Tailwind class names for smaller output. Default: `false`

#### CSS Generation Options

- **`css.noPreflight`** (boolean): Disable Tailwind's preflight/reset styles. Default: `false`
- **`css.minify`** (boolean): Minify the generated CSS. Default: `true` in production

#### Debug Options

- **`debug`** (boolean): Enable detailed debug logging. Default: `false`
- **`keepTempDir`** (boolean): Preserve temporary directory after build. Default: `false`

#### Output Options

- **`manifestFilename`** (string | false): Filename for the metadata manifest (e.g., `'tailwind.manifest.json'`). Set to `false` to disable manifest generation. Default: `'tailwind.manifest.json'`
  - The manifest contains all extracted Tailwind classes and processing statistics
  - Useful for SSR engines to avoid regenerating classes already in the CSS file
  - Has a stable, predictable filename for easy integration with other tools

## Platform-Specific Binaries

The package includes pre-built binaries for common platforms. If you're distributing your own package that includes tailwind-extractor, you can specify the path to platform-specific binaries:

```javascript
const os = require('os');
const path = require('path');

function getTailwindExtractorPath() {
  const platform = os.platform();
  const arch = os.arch();

  const archMap = {
    'x64': 'x86_64',
    'arm64': 'aarch64',
  };

  const binaryArch = archMap[arch] || arch;
  const binaryName = platform === 'win32'
    ? 'tailwind-extractor-cli.exe'
    : 'tailwind-extractor-cli';

  return path.join(__dirname, 'bins', `${binaryArch}-${platform}`, binaryName);
}

// Use in configuration
new TailwindExtractor({
  tailwindExtractorPath: getTailwindExtractorPath(),
  // ... other options
})
```

## How It Works

1. **Transformation Phase**: The loader processes each JavaScript/TypeScript file, extracting Tailwind classes and replacing them with optimized versions
2. **Metadata Collection**: Extracted classes are collected in a temporary metadata file
3. **CSS Generation**: After all files are processed, the plugin generates optimized CSS from the collected metadata
4. **Asset Emission**: The generated CSS is added to the build output with a content-based hash

## Environment Variables

- `NODE_ENV=production`: Automatically enables CSS minification
- `DEBUG=true`: Enable debug logging for troubleshooting

## Examples

### Production Configuration

```javascript
new TailwindExtractor({
  transform: {
    obfuscate: true,  // Obfuscate classes for smaller builds
  },
  css: {
    minify: true,     // Minify CSS
    noPreflight: true, // Skip reset if you have your own
  }
})
```

### Development Configuration

```javascript
new TailwindExtractor({
  css: {
    minify: false,    // Keep CSS readable
  },
  debug: true,        // Enable debug logs
  keepTempDir: true,  // Keep temp files for inspection
})
```

### ReScript/React Project

```javascript
new TailwindExtractor({
  test: /\.(res\.mjs|js|jsx)$/,  // Include ReScript output
  include: /src/,                 // Only process src directory
  css: {
    noPreflight: false,           // Include Tailwind resets
  }
})
```

### SSR Integration with Manifest

```javascript
new TailwindExtractor({
  manifestFilename: 'tailwind.manifest.json', // Predictable filename for SSR
})

// In your SSR engine, read the manifest to know which classes are already included:
const manifest = JSON.parse(fs.readFileSync('dist/tailwind.manifest.json'));
const existingClasses = new Set(manifest.classes);

// Skip generating CSS for classes already in the manifest
if (!existingClasses.has(className)) {
  // Generate CSS for this new class
}
```

## Troubleshooting

### Debug Mode

Enable debug mode to see what's happening:

```javascript
new TailwindExtractor({
  debug: true,
  keepTempDir: true,  // Inspect generated metadata
})
```

### No CSS Output

If no CSS is generated:
1. Check that your files match the `test` pattern
2. Ensure files aren't excluded by the `exclude` pattern
3. Verify that files actually contain Tailwind classes
4. Enable debug mode to see which files are processed

### Binary Not Found

If the CLI binary isn't found:
1. Check that the package installed correctly
2. Verify the binary exists in `node_modules/tailwind-extractor/bins/`
3. Explicitly set `tailwindExtractorPath` if using a custom location

## License

MIT

## Development

### Prerequisites

- Node.js >= 16.0.0
- npm or yarn or pnpm
- Rust toolchain (for building the CLI from source)
- Nix (optional, for building static binaries)

### Setup

```bash
# Install dependencies
npm install

# Run tests
npm test

# Run tests in watch mode
npm test:watch
```

### Building the CLI Binary

The Rust CLI source is included in this repository. During development, you can build it with Cargo like any normal Rust crate:

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run CLI directly
cargo run -- transform output.json < input.js
```

For distribution, we provide pre-built static binaries. To rebuild them for all platforms:

```bash
# Using Nix (work in progress)
nix run .#build-static-binaries
```

Binaries are automatically rebuilt and published on each release via GitHub Actions.

### Testing

```bash
# Run all tests
npm test

# Run specific test suite
npm test -- plugin.test.js

# Run with coverage
npm test:coverage

# Debug mode
npm test:debug
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [GitHub Repository](https://github.com/reacrust/tailwind-rs-extractor)
- [Issue Tracker](https://github.com/reacrust/tailwind-rs-extractor/issues)