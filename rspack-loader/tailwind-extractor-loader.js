const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * RSpack/Webpack loader for transforming Tailwind classes in source files.
 * 
 * This loader runs BEFORE bundling, transforming Tailwind utility classes
 * in JavaScript/TypeScript source files to match the SSR transformations.
 * 
 * Unlike the plugin which runs on bundled code, this loader processes
 * raw source files, ensuring transformations work correctly.
 */
module.exports = function tailwindExtractorLoader(source, map, meta) {
  const callback = this.async();
  const options = this.getOptions() || {};
  
  // Skip if file shouldn't be transformed
  if (!shouldTransformSource(this.resourcePath, options)) {
    return callback(null, source, map, meta);
  }
  
  // Find the CLI binary
  const cliPath = options.cliPath || findCliBinary();
  
  if (options.verbose) {
    console.log(`[tailwind-loader] Transforming: ${this.resourcePath}`);
  }
  
  // Transform using the CLI
  transformSource(source, cliPath, options)
    .then(transformedSource => {
      if (transformedSource !== source && options.verbose) {
        console.log(`[tailwind-loader] Transformed ${this.resourcePath}`);
      }
      callback(null, transformedSource, map, meta);
    })
    .catch(err => {
      console.error(`[tailwind-loader] Error transforming ${this.resourcePath}:`, err.message);
      // Don't fail the build, just return original source
      callback(null, source, map, meta);
    });
};

/**
 * Check if this source file should be transformed.
 * 
 * @param {string} resourcePath - Path to the source file
 * @param {Object} options - Loader options
 * @returns {boolean} True if should transform
 */
function shouldTransformSource(resourcePath, options) {
  // Only transform JavaScript/TypeScript files
  if (!/\.(m?js|jsx|ts|tsx)$/.test(resourcePath)) {
    return false;
  }
  
  // Skip node_modules by default
  if (/node_modules/.test(resourcePath) && !options.includeNodeModules) {
    return false;
  }
  
  // Skip test files by default
  if (/\.(test|spec)\.(js|ts)x?$/.test(resourcePath) && !options.includeTests) {
    return false;
  }
  
  // Check include patterns if provided
  if (options.include) {
    const patterns = Array.isArray(options.include) ? options.include : [options.include];
    const matches = patterns.some(pattern => {
      if (pattern instanceof RegExp) {
        return pattern.test(resourcePath);
      }
      return resourcePath.includes(pattern);
    });
    if (!matches) return false;
  }
  
  // Check exclude patterns if provided
  if (options.exclude) {
    const patterns = Array.isArray(options.exclude) ? options.exclude : [options.exclude];
    const matches = patterns.some(pattern => {
      if (pattern instanceof RegExp) {
        return pattern.test(resourcePath);
      }
      return resourcePath.includes(pattern);
    });
    if (matches) return false;
  }
  
  return true;
}

/**
 * Find the tailwind-extractor-cli binary.
 * 
 * @returns {string} Path to the CLI binary
 */
function findCliBinary() {
  const possiblePaths = [
    // Relative to the loader directory
    path.resolve(__dirname, '../../../target/debug/tailwind-extractor-cli'),
    path.resolve(__dirname, '../../../target/release/tailwind-extractor-cli'),
    // Alternative cargo target location
    path.resolve(__dirname, '../../target/debug/tailwind-extractor-cli'),
    path.resolve(__dirname, '../../target/release/tailwind-extractor-cli'),
    // In PATH
    'tailwind-extractor-cli',
  ];

  for (const cliPath of possiblePaths) {
    if (cliPath === 'tailwind-extractor-cli' || fs.existsSync(cliPath)) {
      return cliPath;
    }
  }

  throw new Error('tailwind-loader: Could not find tailwind-extractor-cli binary. Please specify cliPath option.');
}

/**
 * Transform source code using the tailwind-extractor CLI.
 * 
 * @param {string} source - Source code to transform
 * @param {string} cliPath - Path to the CLI binary
 * @param {Object} options - Transformation options
 * @returns {Promise<string>} Transformed source code
 */
function transformSource(source, cliPath, options) {
  return new Promise((resolve, reject) => {
    // Use the 'pipe --transform' command to transform JavaScript
    const args = ['pipe', '--transform'];
    
    // Note: The --transform flag tells the CLI to transform class names
    // in JavaScript code, not extract CSS
    
    const child = spawn(cliPath, args, {
      timeout: options.timeout || 10000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    
    // Set up timeout handler
    const timeoutId = setTimeout(() => {
      timedOut = true;
      child.kill('SIGTERM');
      reject(new Error(`Transformation timed out after ${options.timeout || 10000}ms`));
    }, options.timeout || 10000);
    
    // Collect stdout
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    
    // Collect stderr
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    
    // Handle process completion
    child.on('close', (code) => {
      clearTimeout(timeoutId);
      
      if (timedOut) {
        return; // Already rejected
      }
      
      if (code !== 0) {
        reject(new Error(`CLI exited with code ${code}: ${stderr}`));
      } else {
        // Return transformed source or original if no transformation
        resolve(stdout || source);
      }
    });
    
    // Handle errors
    child.on('error', (err) => {
      clearTimeout(timeoutId);
      reject(err);
    });
    
    // Write source to stdin
    child.stdin.write(source);
    child.stdin.end();
  });
}

/**
 * Loader options:
 * 
 * @param {string} [cliPath] - Path to tailwind-extractor-cli binary
 * @param {Array<string|RegExp>} [include] - Patterns to include
 * @param {Array<string|RegExp>} [exclude] - Patterns to exclude
 * @param {boolean} [includeNodeModules=false] - Transform node_modules files
 * @param {boolean} [includeTests=false] - Transform test files
 * @param {boolean} [verbose=false] - Enable verbose logging
 * @param {number} [timeout=10000] - Transformation timeout in ms
 */
module.exports.raw = false;