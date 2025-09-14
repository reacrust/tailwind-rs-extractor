/**
 * Example Rspack configuration showing tailwind-extractor integration
 *
 * This demonstrates the simplified, unified plugin approach that automatically
 * configures both the loader and plugin for Tailwind CSS extraction.
 */

const path = require('path');
const os = require('os');
const TailwindExtractor = require('tailwind-extractor'); // or require('.') if running locally

// Detect the appropriate binary based on platform and architecture
function getTailwindExtractorPath() {
  const platform = os.platform();
  const arch = os.arch();

  // Map Node.js arch names to our binary directory names
  const archMap = {
    'x64': 'x86_64',
    'arm64': 'aarch64',
  };

  const binaryArch = archMap[arch] || arch;
  const binaryName = platform === 'win32' ? 'tailwind-extractor-cli.exe' : 'tailwind-extractor-cli';

  // Path to the platform-specific binary
  return path.join(__dirname, 'bins', `${binaryArch}-${platform}`, binaryName);
}

module.exports = {
  entry: './src/index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].[contenthash].js'
  },
  plugins: [
    // Tailwind Extractor Unified Plugin - handles both transformation and CSS generation
    new TailwindExtractor({
      // File pattern options (for the loader)
      test: /\.(js|jsx|ts|tsx|mjs)$/,
      exclude: /node_modules/,
      // include: /src/, // Optional: only process files in src/

      // Transform options (passed to the loader)
      transform: {
        enabled: true,
        obfuscate: process.env.NODE_ENV === 'production',
      },

      // CSS generation options (passed to the plugin)
      css: {
        noPreflight: false, // Set to true to disable Tailwind's reset/preflight styles
        minify: process.env.NODE_ENV === 'production',
      },

      // Debug and cleanup options
      keepTempDir: false, // Set to true to preserve temp directory for debugging
      debug: process.env.DEBUG === 'true', // Enable debug logging

      // Path to the platform-specific CLI binary
      // The plugin will use 'tailwind-extractor-cli' from PATH if not specified
      // tailwindExtractorPath: getTailwindExtractorPath(),
    }),
    
    // HtmlRspackPlugin would go here to auto-inject the CSS
    // The generated CSS file will be named tailwind.[contenthash].css
  ],
  
  // Development server configuration
  devServer: {
    static: './dist',
    hot: true
  }
};