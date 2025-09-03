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
 * 
 * Additionally, this loader extracts Tailwind classes and attaches them
 * to the module metadata for the plugin to aggregate.
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
    console.log(`[tailwind-loader] Processing: ${this.resourcePath}`);
  }
  
  // Extract Tailwind classes from the source before transformation
  const extractedClasses = extractTailwindClasses(source);
  
  if (extractedClasses.length > 0) {
    // Attach extracted classes to the module metadata
    // Try multiple approaches to ensure compatibility
    try {
      // Approach 1: Direct attachment to module
      if (this._module && typeof this._module === 'object') {
        // Safe assignment avoiding Symbol conversion issues
        try {
          this._module.tailwindClasses = extractedClasses;
        } catch (innerErr) {
          // Fallback: use defineProperty if direct assignment fails
          try {
            Object.defineProperty(this._module, 'tailwindClasses', {
              value: extractedClasses,
              writable: true,
              enumerable: false,
              configurable: true
            });
          } catch (defErr) {
            // Ignore if we can't set on _module directly
          }
        }
        
        // Approach 2: Also add to buildInfo for webpack compatibility
        try {
          if (!this._module.buildInfo || typeof this._module.buildInfo !== 'object') {
            this._module.buildInfo = {};
          }
          this._module.buildInfo.tailwindClasses = extractedClasses;
        } catch (buildErr) {
          // Ignore buildInfo errors
        }
        
        // Approach 3: Also add metadata object
        try {
          this._module.tailwindMetadata = {
            resource: this.resourcePath,
            classes: extractedClasses,
            transformed: true
          };
        } catch (metaErr) {
          // Fallback: use defineProperty
          try {
            Object.defineProperty(this._module, 'tailwindMetadata', {
              value: {
                resource: this.resourcePath,
                classes: extractedClasses,
                transformed: true
              },
              writable: true,
              enumerable: false,
              configurable: true
            });
          } catch (defErr) {
            // Ignore if we can't set metadata
          }
        }
        
        if (options.verbose) {
          console.log(`[tailwind-loader] Extracted ${extractedClasses.length} Tailwind classes from ${this.resourcePath}`);
          if (extractedClasses.length <= 10) {
            console.log(`[tailwind-loader] Classes:`, extractedClasses);
          }
        }
      } else if (options.verbose) {
        console.log(`[tailwind-loader] Warning: _module not available for metadata attachment in ${this.resourcePath}`);
      }
    } catch (err) {
      if (options.verbose) {
        // Safely convert error to string
        const errMsg = err && err.message ? err.message : String(err);
        console.log(`[tailwind-loader] Warning: Failed to attach metadata:`, errMsg);
      }
    }
  } else if (options.verbose) {
    console.log(`[tailwind-loader] No Tailwind classes found in ${this.resourcePath}`);
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
 * Extract Tailwind classes from source code using pattern matching.
 * 
 * This function identifies potential Tailwind utility classes using a comprehensive
 * regex pattern that matches the structure of Tailwind class names.
 * 
 * @param {string} source - Source code to analyze
 * @returns {Array<string>} Array of unique Tailwind class names found
 */
function extractTailwindClasses(source) {
  // Comprehensive Tailwind class pattern
  // This pattern matches:
  // - Standard utilities: bg-red-500, text-lg, p-4, etc.
  // - Responsive modifiers: sm:, md:, lg:, xl:, 2xl:
  // - State modifiers: hover:, focus:, active:, disabled:, etc.
  // - Dark mode: dark:
  // - Arbitrary values: w-[100px], text-[#ff0000]
  // - Negative values: -mt-4, -translate-x-1/2
  // - Fractional values: w-1/2, h-3/4
  // - Important modifier: !bg-red-500
  
  const patterns = [
    // Standard Tailwind utilities with optional modifiers and values
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:!?-?(?:container|sr-only|not-sr-only|static|fixed|absolute|relative|sticky|inset|top|right|bottom|left|z|flex|inline-flex|table|inline-table|grid|inline-grid|hidden|block|inline-block|flow-root|contents|list-item|float|clear|object|overflow|overscroll|visible|invisible|isolate|isolation|break|box|decoration|indent|align|justify|place|self|items|content|gap|order|col|row|auto|basis|grow|shrink|w|h|min-w|min-h|max-w|max-h|aspect|p|px|py|pt|pr|pb|pl|ps|pe|m|mx|my|mt|mr|mb|ml|ms|me|space|divide|border|rounded|ring|shadow|opacity|mix-blend|bg|from|via|to|text|font|leading|tracking|line|list|placeholder|caret|accent|appearance|cursor|outline|pointer-events|resize|scroll|snap|touch|select|will-change|fill|stroke|animate|transition|duration|delay|ease|scale|rotate|translate|skew|origin|blur|brightness|contrast|grayscale|hue-rotate|invert|saturate|sepia|backdrop)(?:-(?:[a-zA-Z]+|[0-9]+(?:\/[0-9]+)?|\[[^\]]+\]))*)\b/g,
    
    // Arbitrary value classes like w-[100px], text-[#ff0000], bg-[url(...)]
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*[a-z]+(?:-[a-z]+)*-\[[^\]]+\]/g,
    
    // Color utilities with shades (e.g., bg-red-500, text-blue-900)
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:bg|text|border|ring|divide|placeholder|from|via|to|accent|fill|stroke)-(?:inherit|current|transparent|black|white|slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-(?:50|100|200|300|400|500|600|700|800|900|950)\b/g,
    
    // Special color values
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:bg|text|border|ring|divide|placeholder|from|via|to|accent|fill|stroke)-(?:inherit|current|transparent|black|white)\b/g,
    
    // Spacing utilities with numeric values
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:p|px|py|pt|pr|pb|pl|ps|pe|m|mx|my|mt|mr|mb|ml|ms|me|space|gap|top|right|bottom|left|inset)-(?:0|px|0\.5|1|1\.5|2|2\.5|3|3\.5|4|5|6|7|8|9|10|11|12|14|16|20|24|28|32|36|40|44|48|52|56|60|64|72|80|96)\b/g,
    
    // Width and height utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:w|h|min-w|min-h|max-w|max-h)-(?:0|px|0\.5|1|1\.5|2|2\.5|3|3\.5|4|5|6|7|8|9|10|11|12|14|16|20|24|28|32|36|40|44|48|52|56|60|64|72|80|96|auto|full|screen|min|max|fit|1\/2|1\/3|2\/3|1\/4|2\/4|3\/4|1\/5|2\/5|3\/5|4\/5|1\/6|2\/6|3\/6|4\/6|5\/6|1\/12|2\/12|3\/12|4\/12|5\/12|6\/12|7\/12|8\/12|9\/12|10\/12|11\/12)\b/g,
    
    // Flexbox and Grid utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:flex|grid|col|row)-(?:1|2|3|4|5|6|7|8|9|10|11|12|auto|span-1|span-2|span-3|span-4|span-5|span-6|span-7|span-8|span-9|span-10|span-11|span-12|span-full|start-1|start-2|start-3|start-4|start-5|start-6|start-7|start-8|start-9|start-10|start-11|start-12|start-13|start-auto|end-1|end-2|end-3|end-4|end-5|end-6|end-7|end-8|end-9|end-10|end-11|end-12|end-13|end-auto|none|row|row-reverse|col|col-reverse|wrap|wrap-reverse|nowrap)\b/g,
    
    // Typography utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:text|font|leading|tracking)-(?:xs|sm|base|lg|xl|2xl|3xl|4xl|5xl|6xl|7xl|8xl|9xl|thin|extralight|light|normal|medium|semibold|bold|extrabold|black|italic|roman|uppercase|lowercase|capitalize|normal-case|underline|overline|line-through|no-underline|tight|snug|normal|relaxed|loose|3|4|5|6|7|8|9|10|none|tighter|wider|left|center|right|justify|start|end)\b/g,
    
    // Border utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:border|divide)-(?:0|2|4|8|x|y|t|r|b|l|s|e|solid|dashed|dotted|double|none)\b/g,
    
    // Rounded utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*rounded(?:-(?:none|sm|md|lg|xl|2xl|3xl|full|t|r|b|l|s|e|tl|tr|br|bl|ss|se|es|ee))?\b/g,
    
    // Shadow utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*shadow(?:-(?:sm|md|lg|xl|2xl|inner|none))?\b/g,
    
    // Display utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:block|inline-block|inline|flex|inline-flex|table|inline-table|table-caption|table-cell|table-column|table-column-group|table-footer-group|table-header-group|table-row-group|table-row|flow-root|grid|inline-grid|contents|list-item|hidden)\b/g,
    
    // Position utilities  
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:static|fixed|absolute|relative|sticky)\b/g,
    
    // Z-index utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*z-(?:0|10|20|30|40|50|auto)\b/g,
    
    // Opacity utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*opacity-(?:0|5|10|20|25|30|40|50|60|70|75|80|90|95|100)\b/g,
    
    // Transition utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:transition|duration|delay|ease)-(?:none|all|colors|opacity|shadow|transform|75|100|150|200|300|500|700|1000|linear|in|out|in-out)\b/g,
    
    // Transform utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*(?:scale|rotate|translate|skew)-(?:x|y|0|50|75|90|95|100|105|110|125|150|1|2|3|6|12|45|90|180|-1|-2|-3|-6|-12|-45|-90|-180|1\/2|1\/3|2\/3|1\/4|2\/4|3\/4|full|-full|-1\/2|-1\/3|-2\/3|-1\/4|-2\/4|-3\/4)\b/g,
    
    // Animation utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*animate-(?:none|spin|ping|pulse|bounce)\b/g,
    
    // Cursor utilities
    /\b(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*cursor-(?:auto|default|pointer|wait|text|move|not-allowed|none|context-menu|progress|cell|crosshair|vertical-text|alias|copy|no-drop|grab|grabbing|all-scroll|zoom-in|zoom-out)\b/g
  ];
  
  const classes = new Set();
  
  // Apply each pattern to extract classes
  patterns.forEach(pattern => {
    let match;
    while ((match = pattern.exec(source)) !== null) {
      // Filter out false positives and ensure valid Tailwind class structure
      const className = match[0];
      
      // Basic validation: must start with a letter or responsive prefix
      if (/^(?:(?:sm|md|lg|xl|2xl|hover|focus|active|disabled|dark|group-hover|peer-checked|first|last|odd|even|focus-within|focus-visible|motion-safe|motion-reduce|print|rtl|ltr):)*[a-z!-]/.test(className)) {
        classes.add(className);
      }
    }
  });
  
  return Array.from(classes);
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