const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Module-level storage for CSS files across all compiler instances
const globalCssFiles = new Set();

/**
 * RSpack plugin for Tailwind CSS extraction using the tailwind-extractor-cli tool.
 * 
 * This plugin runs during the processAssets hook to extract Tailwind CSS classes
 * from compiled JavaScript files and generates optimized CSS with content hashing.
 */
class TailwindExtractorPlugin {
  /**
   * Create a new TailwindExtractorPlugin instance.
   * 
   * @param {Object} options - Configuration options
   * @param {Array<string>} options.input - Input file patterns to scan (e.g., glob patterns)
   * @param {string} [options.outputCss] - (Deprecated) Path where the generated CSS file will be written
   * @param {string} [options.outputManifest] - (Optional) Path where the JSON manifest will be written
   * @param {string} [options.cliPath] - Path to the tailwind-extractor-cli binary
   * @param {string} [options.config] - Path to configuration file (YAML format)
   * @param {boolean} [options.obfuscate=false] - Enable obfuscation of Tailwind class names
   * @param {boolean} [options.minify=false] - Enable minification of the output CSS
   * @param {boolean} [options.verbose=false] - Enable verbose output
   * @param {number} [options.timeout=30000] - Timeout for CLI execution in milliseconds
   * @param {number} [options.jobs] - Number of parallel threads to use
   * @param {Array<string>} [options.exclude] - Patterns to exclude from scanning
   * @param {boolean} [options.dryRun=false] - Perform extraction but don't write output files
   * @param {boolean} [options.preflight=true] - Enable generation of Tailwind preflight/reset CSS
   */
  constructor(options = {}) {
    // Validate required options
    if (!options.input || !Array.isArray(options.input) || options.input.length === 0) {
      throw new Error('TailwindExtractorPlugin: options.input must be a non-empty array of file patterns');
    }

    this.options = {
      input: options.input,
      outputCss: options.outputCss, // Deprecated, kept for backward compatibility
      outputManifest: options.outputManifest,
      cliPath: options.cliPath || this.findCliBinary(),
      config: options.config,
      obfuscate: Boolean(options.obfuscate),
      minify: Boolean(options.minify),
      verbose: Boolean(options.verbose),
      timeout: options.timeout || 30000,
      jobs: options.jobs,
      exclude: options.exclude || [],
      dryRun: Boolean(options.dryRun),
      preflight: options.preflight !== undefined ? Boolean(options.preflight) : true, // Default to true for backward compatibility
    };
    
    // Track cleanup handlers
    this.cleanupHandlers = new Set();
  }

  /**
   * Find the tailwind-extractor-cli binary.
   * Looks in common locations relative to the plugin directory.
   * 
   * @returns {string} Path to the CLI binary
   */
  findCliBinary() {
    const possiblePaths = [
      // Relative to the plugin directory (same repo structure)
      path.resolve(__dirname, '../../../target/debug/tailwind-extractor-cli'),
      path.resolve(__dirname, '../../../target/release/tailwind-extractor-cli'),
      // In PATH
      'tailwind-extractor-cli',
    ];

    for (const cliPath of possiblePaths) {
      if (cliPath === 'tailwind-extractor-cli' || fs.existsSync(cliPath)) {
        return cliPath;
      }
    }

    throw new Error('TailwindExtractorPlugin: Could not find tailwind-extractor-cli binary. Please specify cliPath option.');
  }

  /**
   * Apply the plugin to the RSpack compiler.
   * 
   * @param {Object} compiler - RSpack compiler instance
   */
  apply(compiler) {
    const pluginName = 'TailwindExtractorPlugin';

    // Build Gate 2.1: Hook Migration - Use processAssets instead of beforeCompile
    compiler.hooks.compilation.tap(pluginName, (compilation) => {
      // Import webpack from compiler instance to get correct version
      const webpack = compiler.webpack || require('@rspack/core');
      
      // Try to hook into HtmlRspackPlugin if available
      try {
        // Try to find HtmlRspackPlugin from the compiler's options
        const HtmlRspackPlugin = compiler.options.plugins
          ?.find(p => p.constructor.name === 'HtmlRspackPlugin')
          ?.constructor;
        
        if (this.options.verbose) {
          console.log(`[${pluginName}] Found HtmlRspackPlugin:`, !!HtmlRspackPlugin);
        }
        
        if (HtmlRspackPlugin && HtmlRspackPlugin.getCompilationHooks) {
          const hooks = HtmlRspackPlugin.getCompilationHooks(compilation);
          
          if (this.options.verbose) {
            console.log(`[${pluginName}] Got HtmlRspackPlugin hooks`);
          }
          
          // Solution: Use beforeEmit hook to inject CSS after all processing is done
          hooks.beforeEmit.tap(pluginName, (data) => {
            // First check compilation.assets for this specific compilation
            const localCssAssets = Object.keys(compilation.assets).filter(name => 
              name.startsWith('tailwind.') && name.endsWith('.css')
            );
            
            // Also check our global set for CSS files from any compilation
            const allCssFiles = [...globalCssFiles, ...localCssAssets];
            const uniqueCssFiles = [...new Set(allCssFiles)];
            
            if (this.options.verbose) {
              console.log(`[${pluginName}] beforeEmit hook called`);
              console.log(`[${pluginName}] Local CSS assets:`, localCssAssets);
              console.log(`[${pluginName}] Global CSS files:`, [...globalCssFiles]);
              console.log(`[${pluginName}] Using CSS files:`, uniqueCssFiles);
            }
            
            if (uniqueCssFiles.length > 0) {
              // Parse the HTML to inject CSS
              let html = data.html;
              
              // Use the most recent CSS file (last in array)
              const cssFile = uniqueCssFiles[uniqueCssFiles.length - 1];
              const cssPath = `/dist/${cssFile}`;
              
              if (!html.includes(cssPath)) {
                // Find the </head> tag and inject before it
                const headEndIndex = html.indexOf('</head>');
                if (headEndIndex !== -1) {
                  const cssLink = `    <link rel="stylesheet" href="${cssPath}">\n`;
                  html = html.slice(0, headEndIndex) + cssLink + html.slice(headEndIndex);
                  data.html = html;
                  
                  if (this.options.verbose) {
                    console.log(`[${pluginName}] Injected CSS link tag into HTML: ${cssFile}`);
                  }
                }
              }
            } else if (this.options.verbose) {
              console.log(`[${pluginName}] No Tailwind CSS files available for injection`);
            }
            
            return data;
          });
        }
      } catch (err) {
        // HtmlRspackPlugin not available or not compatible - that's fine
        if (this.options.verbose) {
          console.log(`[${pluginName}] HtmlRspackPlugin integration error:`, err.message);
        }
      }
      
      compilation.hooks.processAssets.tapPromise({
        name: pluginName,
        stage: webpack.Compilation.PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE,
      }, async (assets) => {
        try {
          if (this.options.verbose) {
            console.log(`[${pluginName}] Starting async Tailwind CSS extraction...`);
          }

          // Build Gate 2.3: Asset Emission - Collect JavaScript content and emit CSS
          const cssFilename = await this.processAssetsAsync(compilation, assets);

          if (cssFilename) {
            // Track the generated CSS file globally
            globalCssFiles.add(cssFilename);
            if (this.options.verbose) {
              console.log(`[${pluginName}] Tailwind CSS extraction completed successfully`);
              console.log(`[${pluginName}] Added to global CSS files:`, cssFilename);
            }
          }
        } catch (error) {
          // Log error but don't fail the build - allow webpack to continue
          console.error(`[${pluginName}] Warning: Tailwind extraction encountered an error:`, error.message);
          console.error(`[${pluginName}] Build will continue without Tailwind CSS generation for this bundle.`);
          // Don't throw - let build continue
        }
      });
    });
  }

  /**
   * Process assets asynchronously and emit CSS with content hashing.
   * 
   * @param {Object} compilation - Webpack compilation object
   * @param {Object} assets - Compilation assets
   * @returns {Promise<string|null>} The filename of the emitted CSS asset, or null
   */
  async processAssetsAsync(compilation, assets) {
    // Build Gate 2.3: Collect JavaScript content from compilation assets
    const jsAssets = Object.keys(assets).filter(name => 
      name.endsWith('.js') || name.endsWith('.mjs')
    );
    
    if (jsAssets.length === 0) {
      if (this.options.verbose) {
        console.log('[TailwindExtractorPlugin] No JavaScript assets found, skipping extraction');
      }
      return null;
    }

    // Combine all JavaScript content
    let combinedJsContent = '';
    for (const assetName of jsAssets) {
      const asset = assets[assetName];
      const source = asset.source();
      if (source) {
        combinedJsContent += source.toString() + '\n';
      }
    }

    if (!combinedJsContent.trim()) {
      if (this.options.verbose) {
        console.log('[TailwindExtractorPlugin] No JavaScript content found in assets');
      }
      return null;
    }

    // Build Gate 2.2: Extract CSS via async stdin/stdout pipe
    const cssContent = await this.extractViaStdin(combinedJsContent);
    
    if (!cssContent || !cssContent.trim()) {
      if (this.options.verbose) {
        console.log('[TailwindExtractorPlugin] No CSS content generated');
      }
      return null;
    }

    // Build Gate 2.3: Emit CSS with content hashing
    const hash = require('crypto')
      .createHash('md5')
      .update(cssContent)
      .digest('hex')
      .substring(0, 8);
    
    const cssFilename = `tailwind.${hash}.css`;
    
    // Use webpack.sources.RawSource for the CSS content
    // Get webpack from compilation context
    const webpack = compilation.compiler.webpack || require('@rspack/core');
    const { RawSource } = webpack.sources;
    compilation.emitAsset(cssFilename, new RawSource(cssContent));
    
    if (this.options.verbose) {
      console.log(`[TailwindExtractorPlugin] Emitted CSS asset: ${cssFilename}`);
    }
    
    // Optionally write manifest if configured
    if (this.options.outputManifest) {
      try {
        const manifestData = {
          cssFile: cssFilename,
          hash: hash,
          timestamp: new Date().toISOString(),
          size: cssContent.length,
        };
        fs.writeFileSync(
          this.options.outputManifest, 
          JSON.stringify(manifestData, null, 2)
        );
      } catch (err) {
        console.warn(`[TailwindExtractorPlugin] Failed to write manifest: ${err.message}`);
      }
    }
    
    return cssFilename;
  }

  /**
   * Build Gate 2.2: Extract CSS using async stdin/stdout pipe.
   * 
   * @param {string} jsContent - JavaScript content to process
   * @returns {Promise<string>} Extracted CSS content
   */
  async extractViaStdin(jsContent) {
    return new Promise((resolve, reject) => {
      const args = ['pipe'];
      
      // Add optional arguments (pipe mode only supports minify and no-preflight)
      if (this.options.minify) {
        args.push('--minify');
      }
      
      // Add no-preflight flag if preflight is disabled
      if (!this.options.preflight) {
        args.push('--no-preflight');
      }
      
      // Note: pipe mode doesn't support these flags, they're ignored:
      // --obfuscate, --verbose, --config, --jobs

      if (this.options.verbose) {
        console.log(`[TailwindExtractorPlugin] Spawning: ${this.options.cliPath} ${args.join(' ')}`);
      }

      const child = spawn(this.options.cliPath, args, {
        timeout: this.options.timeout,
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      let stdout = '';
      let stderr = '';
      let timedOut = false;

      // Set up timeout handler
      const timeoutId = setTimeout(() => {
        timedOut = true;
        child.kill('SIGTERM');
        reject(new Error(`Tailwind extraction timed out after ${this.options.timeout}ms`));
      }, this.options.timeout);

      // Cleanup function to ensure child process is terminated
      const cleanup = () => {
        clearTimeout(timeoutId);
        if (!child.killed) {
          child.kill('SIGTERM');
        }
      };

      // Register cleanup handlers
      const registerCleanup = () => {
        const handlers = ['exit', 'SIGINT', 'SIGTERM'].map(event => {
          const handler = () => {
            cleanup();
            this.cleanupHandlers.delete(handler);
          };
          this.cleanupHandlers.add(handler);
          process.once(event, handler);
          return handler;
        });
      };
      
      registerCleanup();

      // Write JavaScript content to stdin
      child.stdin.on('error', (err) => {
        if (!timedOut) {
          cleanup();
          reject(new Error(`Failed to write to stdin: ${err.message}`));
        }
      });

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('error', (err) => {
        if (!timedOut) {
          cleanup();
          reject(new Error(`Failed to spawn tailwind-extractor-cli: ${err.message}`));
        }
      });

      child.on('close', (code) => {
        cleanup();
        
        if (timedOut) {
          return; // Already rejected due to timeout
        }
        
        if (code !== 0) {
          reject(new Error(`Tailwind extraction failed with exit code ${code}: ${stderr}`));
        } else {
          if (this.options.verbose && stderr) {
            console.log(`[TailwindExtractorPlugin] CLI output: ${stderr}`);
          }
          resolve(stdout);
        }
      });

      // Write content and close stdin
      child.stdin.write(jsContent);
      child.stdin.end();
    });
  }

  /**
   * Execute the tailwind-extractor-cli with the configured options.
   * @deprecated Use extractViaStdin for async processing
   */
  extract() {
    console.warn('[TailwindExtractorPlugin] extract() is deprecated. Plugin now uses async processing.');
  }

  /**
   * Build command line arguments for the tailwind-extractor-cli.
   * @deprecated No longer needed with stdin/stdout pipe mode
   */
  buildArgs() {
    console.warn('[TailwindExtractorPlugin] buildArgs() is deprecated. Plugin now uses pipe mode.');
    return '';
  }

  /**
   * Escape a command line argument to prevent shell injection.
   * @deprecated No longer needed with spawn API
   */
  escapeArg(arg) {
    return arg;
  }
}

module.exports = TailwindExtractorPlugin;