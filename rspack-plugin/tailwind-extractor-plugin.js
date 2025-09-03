const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Module-level storage for CSS files across all compiler instances
const globalCssFiles = new Set();

/**
 * RSpack plugin for Tailwind CSS extraction using the tailwind-extractor-cli tool.
 * 
 * This plugin aggregates Tailwind classes from module metadata and generates
 * optimized CSS with content hashing in a single pass.
 */
class TailwindExtractorPlugin {
  /**
   * Create a new TailwindExtractorPlugin instance.
   * 
   * @param {Object} options - Configuration options
   * @param {Array<string>} options.input - Input file patterns to scan (kept for compatibility)
   * @param {string} [options.outputManifest] - (Optional) Path where the JSON manifest will be written
   * @param {string} [options.cliPath] - Path to the tailwind-extractor-cli binary
   * @param {string} [options.config] - Path to configuration file (YAML format)
   * @param {boolean} [options.minify=false] - Enable minification of the output CSS
   * @param {boolean} [options.verbose=false] - Enable verbose output
   * @param {number} [options.timeout=30000] - Timeout for CLI execution in milliseconds
   * @param {boolean} [options.preflight=true] - Enable generation of Tailwind preflight/reset CSS
   */
  constructor(options = {}) {
    this.options = {
      input: options.input || [], // Kept for backward compatibility
      outputManifest: options.outputManifest,
      cliPath: options.cliPath || this.findCliBinary(),
      config: options.config,
      minify: Boolean(options.minify),
      verbose: Boolean(options.verbose),
      timeout: options.timeout || 30000,
      preflight: options.preflight !== undefined ? Boolean(options.preflight) : true,
    };
    
    // Storage for collected Tailwind classes
    this.collectedClasses = new Set();
    
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

    throw new Error('TailwindExtractorPlugin: Could not find tailwind-extractor-cli binary. Please specify cliPath option.');
  }

  /**
   * Apply the plugin to the RSpack compiler.
   * 
   * @param {Object} compiler - RSpack compiler instance
   */
  apply(compiler) {
    const pluginName = 'TailwindExtractorPlugin';

    compiler.hooks.compilation.tap(pluginName, (compilation) => {
      // Import webpack from compiler instance to get correct version
      const webpack = compiler.webpack || require('@rspack/core');
      
      // Hook into module processing to collect Tailwind classes
      compilation.hooks.finishModules.tap(pluginName, (modules) => {
        if (this.options.verbose) {
          console.log(`[${pluginName}] Collecting Tailwind classes from modules...`);
        }
        
        // Clear previous collection for this compilation
        this.collectedClasses.clear();
        
        // Iterate through all modules and collect their Tailwind metadata
        for (const module of modules) {
          // Check for tailwindClasses directly on the module
          if (module.tailwindClasses) {
            if (Array.isArray(module.tailwindClasses)) {
              module.tailwindClasses.forEach(className => {
                this.collectedClasses.add(className);
              });
              
              if (this.options.verbose) {
                console.log(`[${pluginName}] Found ${module.tailwindClasses.length} classes in module:`, module.resource || module.identifier?.());
              }
            }
          }
          
          // Also check for tailwindMetadata with classes property
          if (module.tailwindMetadata?.classes) {
            if (Array.isArray(module.tailwindMetadata.classes)) {
              module.tailwindMetadata.classes.forEach(className => {
                this.collectedClasses.add(className);
              });
              
              if (this.options.verbose) {
                console.log(`[${pluginName}] Found ${module.tailwindMetadata.classes.length} classes in module metadata:`, module.resource || module.identifier?.());
              }
            }
          }
          
          // Check for buildInfo.tailwindClasses (webpack loader metadata pattern)
          if (module.buildInfo?.tailwindClasses) {
            if (Array.isArray(module.buildInfo.tailwindClasses)) {
              module.buildInfo.tailwindClasses.forEach(className => {
                this.collectedClasses.add(className);
              });
              
              if (this.options.verbose) {
                console.log(`[${pluginName}] Found ${module.buildInfo.tailwindClasses.length} classes in buildInfo:`, module.resource || module.identifier?.());
              }
            }
          }
        }
        
        if (this.options.verbose) {
          console.log(`[${pluginName}] Total unique Tailwind classes collected:`, this.collectedClasses.size);
          if (this.collectedClasses.size > 0 && this.collectedClasses.size <= 20) {
            // Show sample of classes if not too many
            console.log(`[${pluginName}] Classes:`, Array.from(this.collectedClasses));
          }
        }
      });
      
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
          
          // Use beforeEmit hook to inject CSS after all processing is done
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
      
      // Generate CSS once from all collected classes
      compilation.hooks.processAssets.tapPromise({
        name: pluginName,
        stage: webpack.Compilation.PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE
      }, async (assets) => {
        try {
          if (this.options.verbose) {
            console.log(`[${pluginName}] Starting Tailwind CSS generation...`);
          }

          // Only generate CSS if we have collected classes
          if (this.collectedClasses.size === 0) {
            if (this.options.verbose) {
              console.log(`[${pluginName}] No Tailwind classes collected, skipping CSS generation`);
            }
            return;
          }

          // Convert collected classes to array for processing
          const classesArray = Array.from(this.collectedClasses);
          
          if (this.options.verbose) {
            console.log(`[${pluginName}] Generating CSS for ${classesArray.length} unique Tailwind classes`);
          }

          // Generate CSS from collected classes
          const cssFilename = await this.generateCSSFromClasses(compilation, classesArray);

          if (cssFilename) {
            // Track the generated CSS file globally
            globalCssFiles.add(cssFilename);
            if (this.options.verbose) {
              console.log(`[${pluginName}] Tailwind CSS generation completed successfully`);
              console.log(`[${pluginName}] Generated CSS file:`, cssFilename);
            }
          }
        } catch (error) {
          // Log error but don't fail the build - allow webpack to continue
          console.error(`[${pluginName}] Warning: Tailwind CSS generation encountered an error:`, error.message);
          console.error(`[${pluginName}] Build will continue without Tailwind CSS generation.`);
          // Don't throw - let build continue
        }
      });
    });
  }

  /**
   * Generate CSS from collected Tailwind classes and emit with content hashing.
   * 
   * @param {Object} compilation - Webpack compilation object
   * @param {Array<string>} classes - Array of Tailwind class names
   * @returns {Promise<string|null>} The filename of the emitted CSS asset, or null
   */
  async generateCSSFromClasses(compilation, classes) {
    if (!classes || classes.length === 0) {
      return null;
    }

    // Create a JavaScript snippet containing all the classes as strings
    // The CLI's pipe mode expects JavaScript code to extract from, not raw class names
    const jsContent = classes.map(cls => `"${cls.replace(/"/g, '\\"')}"`).join(', ');
    const jsSnippet = `const classes = [${jsContent}];`;
    
    if (this.options.verbose) {
      console.log(`[TailwindExtractorPlugin] Generating CSS from JavaScript with ${classes.length} classes`);
      // Log a sample of the JS for debugging
      if (jsSnippet.length < 500) {
        console.log(`[TailwindExtractorPlugin] JS snippet: ${jsSnippet}`);
      } else {
        console.log(`[TailwindExtractorPlugin] JS snippet (truncated): ${jsSnippet.substring(0, 500)}...`);
      }
    }
    
    // Generate CSS using the CLI in pipe mode
    const cssContent = await this.generateViaStdin(jsSnippet);
    
    if (!cssContent || !cssContent.trim()) {
      if (this.options.verbose) {
        console.log('[TailwindExtractorPlugin] No CSS content generated');
      }
      return null;
    }

    // Generate content hash for the CSS
    const hash = require('crypto')
      .createHash('md5')
      .update(cssContent)
      .digest('hex')
      .substring(0, 8);
    
    const cssFilename = `tailwind.${hash}.css`;
    
    // Use webpack.sources.RawSource for the CSS content
    const webpack = compilation.compiler.webpack || require('@rspack/core');
    const { RawSource } = webpack.sources;
    compilation.emitAsset(cssFilename, new RawSource(cssContent));
    
    if (this.options.verbose) {
      console.log(`[TailwindExtractorPlugin] Emitted CSS asset: ${cssFilename}`);
      console.log(`[TailwindExtractorPlugin] CSS size: ${cssContent.length} bytes`);
    }
    
    // Optionally write manifest if configured
    if (this.options.outputManifest) {
      try {
        const manifestData = {
          cssFile: cssFilename,
          hash: hash,
          timestamp: new Date().toISOString(),
          size: cssContent.length,
          classCount: classes.length
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
   * Generate CSS using the CLI in pipe mode (without --transform).
   * 
   * @param {string} classesContent - Tailwind classes (one per line)
   * @returns {Promise<string>} Generated CSS content
   */
  async generateViaStdin(classesContent) {
    return new Promise((resolve, reject) => {
      const args = ['pipe'];
      
      // Add optional arguments
      if (this.options.minify) {
        args.push('--minify');
      }
      
      // Add no-preflight flag if preflight is disabled
      if (!this.options.preflight) {
        args.push('--no-preflight');
      }

      if (this.options.verbose) {
        console.log(`[TailwindExtractorPlugin] Spawning: ${this.options.cliPath} ${args.join(' ')}`);
        // Log the actual content being sent
        if (classesContent.length < 200) {
          console.log(`[TailwindExtractorPlugin] Stdin content: ${classesContent}`);
        } else {
          console.log(`[TailwindExtractorPlugin] Stdin content (${classesContent.length} chars): ${classesContent.substring(0, 200)}...`);
        }
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
        reject(new Error(`Tailwind CSS generation timed out after ${this.options.timeout}ms`));
      }, this.options.timeout);

      // Cleanup function
      const cleanup = () => {
        clearTimeout(timeoutId);
        if (!child.killed) {
          child.kill('SIGTERM');
        }
      };

      // Register cleanup handlers for process termination
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
          reject(new Error(`Tailwind CSS generation failed with exit code ${code}: ${stderr}`));
        } else {
          if (this.options.verbose && stderr) {
            console.log(`[TailwindExtractorPlugin] CLI output: ${stderr}`);
          }
          resolve(stdout);
        }
      });

      // Write classes and close stdin
      child.stdin.write(classesContent);
      child.stdin.end();
    });
  }
}

module.exports = TailwindExtractorPlugin;