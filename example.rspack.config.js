/**
 * Example Rspack configuration showing tailwind-extractor integration
 * 
 * This demonstrates how to use the loader and plugin together for
 * automatic Tailwind CSS extraction and generation during build.
 */

const path = require('path');
const TailwindExtractorPlugin = require('./rspack-plugin/tailwind-extractor-plugin');

module.exports = {
  entry: './src/index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].[contenthash].js'
  },
  module: {
    rules: [
      {
        // Apply to all JavaScript/TypeScript files
        test: /\.(js|jsx|ts|tsx|mjs)$/,
        exclude: /node_modules/,
        use: [
          // Other loaders (e.g., babel-loader) would go here first
          
          // Tailwind extractor loader must be last
          {
            loader: path.resolve(__dirname, 'rspack-loader/tailwind-extractor-loader.js'),
            options: {
              // Enable obfuscation in production
              obfuscate: process.env.NODE_ENV === 'production',
              
              // Enable/disable transformation
              enabled: true,
              
              // Optional: Custom temp directory
              // tempDir: path.resolve(__dirname, '.tailwind-temp')
            }
          }
        ]
      }
    ]
  },
  plugins: [
    // Tailwind Extractor Plugin - generates CSS from collected metadata
    new TailwindExtractorPlugin({
      // Disable preflight/reset styles if not needed
      noPreflight: false,
      
      // Minify CSS in production
      minify: process.env.NODE_ENV === 'production',
      
      // Clean up temp files after processing (default: true)
      cleanupTempFiles: true,
      
      // Optional: Use same custom temp directory as loader
      // tempDir: path.resolve(__dirname, '.tailwind-temp')
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