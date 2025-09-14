/**
 * Tailwind Extractor Unified Plugin for RSpack/Webpack
 *
 * This plugin automatically configures both the loader and plugin components
 * to extract and process Tailwind CSS classes during the build process.
 */

const path = require('path');
const fs = require('fs');
const os = require('os');
const { spawn } = require('child_process');
const crypto = require('crypto');

// HtmlRspackPlugin will be found from the compiler's plugin instances

class TailwindExtractorPlugin {
  constructor(options = {}) {
    // Separate options into categories
    this.options = {
      // File matching options
      test: options.test || /\.(js|jsx|ts|tsx|mjs)$/,
      exclude: options.exclude || /node_modules/,
      include: options.include,

      // Transform options (for loader)
      transform: {
        enabled: options.transform?.enabled !== false,
        obfuscate: options.transform?.obfuscate || options.obfuscate || false,
        ...options.transform
      },

      // CSS generation options (for plugin)
      css: {
        noPreflight: options.css?.noPreflight || options.noPreflight || false,
        minify: options.css?.minify !== undefined ? options.css.minify :
                (options.minify !== undefined ? options.minify : process.env.NODE_ENV === 'production'),
        obfuscate: options.transform?.obfuscate || options.obfuscate || false,
        ...options.css
      },

      // CSS injection options
      injectCSS: options.injectCSS !== undefined ? options.injectCSS : 'link',

      // CSP options
      csp: {
        nonce: options.csp?.nonce || null,
        ...options.csp
      },

      // Debug and cleanup options
      keepTempDir: options.keepTempDir || false,
      tailwindExtractorPath: options.tailwindExtractorPath || this.getDefaultBinaryPath(),
      debug: options.debug || false,

      // Manifest output options
      manifestFilename: options.manifestFilename !== undefined ? options.manifestFilename : 'tailwind.manifest.json'
    };

    // Internal state - temp directory will be created on first use
    this.tempDir = null;
    this.metadataFile = null;
    this.loaderPath = path.resolve(__dirname, 'lib', 'loader.js');

    // Track generated CSS for injection
    this.generatedCssFilename = null;
    this.generatedCssContent = null;
  }

  /**
   * Get the default binary path based on platform and architecture
   */
  getDefaultBinaryPath() {
    const platform = os.platform();
    const arch = os.arch();

    // Map Node.js arch names to our binary directory names
    const archMap = {
      'x64': 'x86_64',
      'arm64': 'aarch64',
    };

    const binaryArch = archMap[arch] || arch;
    const binaryName = platform === 'win32' ? 'tailwind-extractor-cli.exe' : 'tailwind-extractor-cli';

    // Always use the platform-specific binary from bins directory
    const binaryPath = path.join(__dirname, 'bins', `${binaryArch}-${platform}`, binaryName);

    if (this.options?.debug) {
      console.log(`[TailwindExtractor] Using binary: ${binaryPath}`);
    }

    return binaryPath;
  }

  /**
   * Create a unique temporary directory for this plugin instance
   */
  ensureTempDir() {
    if (!this.tempDir) {
      const prefix = path.join(os.tmpdir(), 'tailwind-extractor-');
      this.tempDir = fs.mkdtempSync(prefix);
      this.metadataFile = path.join(this.tempDir, 'metadata.json');

      if (this.options.debug) {
        console.log(`[TailwindExtractor] Created temp directory: ${this.tempDir}`);
      }
    }
    return this.tempDir;
  }

  apply(compiler) {
    const pluginName = 'TailwindExtractorPlugin';

    // Step 1: Register the loader
    this.registerLoader(compiler);

    // Step 2: Apply the plugin logic
    this.applyPlugin(compiler, pluginName);
  }

  registerLoader(compiler) {
    // Ensure module.rules exists
    compiler.options.module = compiler.options.module || {};
    compiler.options.module.rules = compiler.options.module.rules || [];

    // Check if loader is already registered
    const hasLoader = compiler.options.module.rules.some(rule => {
      if (!rule.use) return false;
      const uses = Array.isArray(rule.use) ? rule.use : [rule.use];
      return uses.some(use => {
        const loader = typeof use === 'string' ? use : use.loader;
        return loader && loader.includes('tailwind-extractor');
      });
    });

    if (!hasLoader) {
      // Create the rule configuration
      const rule = {
        test: this.options.test,
        exclude: this.options.exclude,
        use: [{
          loader: this.loaderPath,
          options: {
            ...this.options.transform,
            // Pass a function to get the temp dir, since it's created lazily
            getTempDir: () => this.ensureTempDir(),
            getMetadataFile: () => {
              this.ensureTempDir();
              return this.metadataFile;
            },
            cliPath: this.options.tailwindExtractorPath,
            debug: this.options.debug
          }
        }]
      };

      // Add include if specified
      if (this.options.include) {
        rule.include = this.options.include;
      }

      // Add the loader rule
      compiler.options.module.rules.push(rule);

      if (this.options.debug) {
        console.log('[TailwindExtractor] Registered loader with options:', rule);
      }
    }
  }

  applyPlugin(compiler, pluginName) {
    // Hook into the compilation
    compiler.hooks.thisCompilation.tap(pluginName, (compilation) => {
      // Set up HTML injection if enabled
      this.setupHtmlInjection(compiler, compilation, pluginName);

      // Process assets after chunks are optimized
      compilation.hooks.processAssets.tapAsync(
        {
          name: pluginName,
          stage: compilation.constructor.PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE
        },
        async (assets, callback) => {
          try {
            // Merge all metadata files first
            const mergedMetadata = this.mergeMetadataFiles();

            if (!mergedMetadata) {
              if (this.options.debug) {
                console.log(`[TailwindExtractor] No metadata found, skipping CSS generation`);
              }
              callback();
              return;
            }

            // Generate CSS from metadata
            const css = await this.generateCSSFromMetadata(mergedMetadata);

            if (css) {
              // Create unique filename with content hash
              const hash = crypto.createHash('md5').update(css).digest('hex').substring(0, 8);
              const filename = `tailwind.${hash}.css`;

              // Store for injection
              this.generatedCssFilename = filename;
              this.generatedCssContent = css;

              // Add CSS asset to compilation (for 'link' mode or no injection)
              if (this.options.injectCSS !== 'inline') {
                assets[filename] = {
                  source: () => css,
                  size: () => css.length
                };
              }

              if (this.options.debug) {
                console.log(`[TailwindExtractor] Generated CSS file: ${filename} (${css.length} bytes)`);
              }
            }

            // Output manifest file if configured
            if (this.options.manifestFilename) {
              const manifestContent = JSON.stringify(mergedMetadata.data, null, 2);
              assets[this.options.manifestFilename] = {
                source: () => manifestContent,
                size: () => manifestContent.length
              };

              if (this.options.debug) {
                console.log(`[TailwindExtractor] Generated manifest file: ${this.options.manifestFilename} (${manifestContent.length} bytes)`);
              }
            }

            callback();
          } catch (error) {
            callback(error);
          }
        }
      );
    });

    // Cleanup on compiler close
    compiler.hooks.done.tap(pluginName, () => {
      if (!this.options.keepTempDir) {
        this.cleanup();
      } else if (this.tempDir && this.options.debug) {
        console.log(`[TailwindExtractor] Keeping temp directory: ${this.tempDir}`);
      }
    });

    // Also cleanup on watchClose for watch mode
    if (compiler.hooks.watchClose) {
      compiler.hooks.watchClose.tap(pluginName, () => {
        if (!this.options.keepTempDir) {
          this.cleanup();
        }
      });
    }
  }

  setupHtmlInjection(compiler, compilation, pluginName) {
    // Only try to hook if injection is enabled
    if (!this.options.injectCSS) {
      return;
    }

    try {
      // Find HtmlRspackPlugin from the compiler's plugin instances
      const htmlPlugin = compiler.options.plugins
        .find(plugin => plugin.constructor.name === 'HtmlRspackPlugin');

      const HtmlRspackPlugin = htmlPlugin?.constructor;

      if (this.options.debug) {
        console.log('[TailwindExtractor] HtmlRspackPlugin instance found:', !!htmlPlugin);
        console.log('[TailwindExtractor] getCompilationHooks available:', !!(HtmlRspackPlugin && HtmlRspackPlugin.getCompilationHooks));
      }

      // Use RSpack's proper hook access method
      if (HtmlRspackPlugin && HtmlRspackPlugin.getCompilationHooks) {
        const htmlHooks = HtmlRspackPlugin.getCompilationHooks(compilation);

        if (this.options.debug) {
          console.log('[TailwindExtractor] htmlHooks available:', !!htmlHooks);
          console.log('[TailwindExtractor] alterAssetTags hook available:', !!(htmlHooks && htmlHooks.alterAssetTags));
        }

        if (htmlHooks && htmlHooks.alterAssetTags) {
          htmlHooks.alterAssetTags.tapAsync(
            pluginName,
            (data, callback) => {
              this.injectCssTags(data);
              callback(null, data);
            }
          );

          if (this.options.debug) {
            console.log('[TailwindExtractor] Successfully hooked into HtmlRspackPlugin');
          }
        } else {
          this.handleNoHtmlPlugin();
        }
      } else {
        this.handleNoHtmlPlugin();
      }
    } catch (err) {
      if (this.options.debug) {
        console.warn('[TailwindExtractor] Could not hook HTML plugin:', err.message);
      }
      this.handleNoHtmlPlugin();
    }
  }

  injectCssTags(data) {
    if (!this.generatedCssContent && !this.generatedCssFilename) {
      return;
    }

    if (this.options.injectCSS === 'link') {
      // Inject as external stylesheet link
      const linkTag = {
        tagName: 'link',
        voidTag: true,
        attributes: {
          href: this.generatedCssFilename,
          rel: 'stylesheet'
        }
      };

      // Add nonce if configured
      if (this.options.csp?.nonce) {
        linkTag.attributes.nonce = this.options.csp.nonce;
      }

      data.assetTags.styles.push(linkTag);

      if (this.options.debug) {
        console.log(`[TailwindExtractor] Injected CSS link: ${this.generatedCssFilename}`);
      }

    } else if (this.options.injectCSS === 'inline') {
      // Inject as inline <style> tag
      const styleTag = {
        tagName: 'style',
        voidTag: false,
        innerHTML: this.generatedCssContent,
        attributes: {
          'data-source': 'tailwind-extractor'
        }
      };

      // Add nonce if configured
      if (this.options.csp?.nonce) {
        styleTag.attributes.nonce = this.options.csp.nonce;
      }

      data.assetTags.styles.push(styleTag);

      if (this.options.debug) {
        console.log(`[TailwindExtractor] Injected inline CSS (${this.generatedCssContent.length} bytes)`);
      }
    }
  }

  handleNoHtmlPlugin() {
    if (this.options.injectCSS === 'link') {
      // CSS file will still be generated, just not auto-injected
      if (this.options.debug) {
        console.warn(
          '[TailwindExtractor] HtmlRspackPlugin not found. CSS file generated but not injected.\n' +
          'Add <link href="tailwind.[hash].css" rel="stylesheet"> to your HTML template.'
        );
      }
    } else if (this.options.injectCSS === 'inline') {
      if (this.options.debug) {
        console.warn(
          '[TailwindExtractor] HtmlRspackPlugin not found. Cannot inject inline CSS.\n' +
          'Consider using injectCSS: "link" mode or install html-rspack-plugin.'
        );
      }
    }
  }

  async generateCSSFromMetadata(mergedMetadata) {
    return new Promise((resolve, reject) => {
      const args = ['generate'];

      // Add CSS generation options
      if (this.options.css.minify) {
        args.push('--minify');
      }
      if (this.options.css.obfuscate) {
        args.push('--obfuscate');
      }
      if (this.options.css.noPreflight) {
        args.push('--no-preflight');
      }

      if (this.options.debug) {
        console.log(`[TailwindExtractor] Running: ${this.options.tailwindExtractorPath} ${args.join(' ')}`);
        console.log(`[TailwindExtractor] Merged metadata from ${mergedMetadata.fileCount} files`);
      }

      // Spawn the tailwind-extractor CLI
      const child = spawn(this.options.tailwindExtractorPath, args, {
        stdio: ['pipe', 'pipe', 'pipe']
      });

      let output = '';
      let errorOutput = '';

      // Pipe merged metadata to stdin
      child.stdin.write(JSON.stringify(mergedMetadata.data));
      child.stdin.end();

      child.stdout.on('data', (data) => {
        output += data.toString();
      });

      child.stderr.on('data', (data) => {
        errorOutput += data.toString();
        if (this.options.debug) {
          console.error(`[TailwindExtractor] CLI stderr: ${data.toString()}`);
        }
      });

      child.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`tailwind-extractor-cli generate failed with code ${code}: ${errorOutput}`));
        } else {
          resolve(output);
        }
      });

      child.on('error', (err) => {
        reject(new Error(`Failed to spawn tailwind-extractor-cli: ${err.message}`));
      });
    });
  }

  mergeMetadataFiles() {
    if (!this.tempDir || !fs.existsSync(this.tempDir)) {
      return null;
    }

    // Find all metadata JSON files in temp directory
    const metadataFiles = fs.readdirSync(this.tempDir)
      .filter(f => f.endsWith('.json'))
      .map(f => path.join(this.tempDir, f));

    if (metadataFiles.length === 0) {
      return null;
    }

    // Merge all metadata files
    const allClasses = new Set();
    const sourceFiles = [];

    for (const file of metadataFiles) {
      try {
        const content = fs.readFileSync(file, 'utf-8');
        const metadata = JSON.parse(content);

        if (metadata.classes) {
          metadata.classes.forEach(cls => allClasses.add(cls));
        }
        if (metadata.sourceFile) {
          sourceFiles.push(metadata.sourceFile);
        }
      } catch (err) {
        if (this.options.debug) {
          console.warn(`[TailwindExtractor] Failed to read metadata file ${file}: ${err.message}`);
        }
      }
    }

    const mergedData = {
      classes: Array.from(allClasses),
      sourceFiles: sourceFiles,
      processedAt: new Date().toISOString(),
      version: "0.2.0",
      stats: {
        originalCount: allClasses.size,
        uniqueCount: allClasses.size,
        filesProcessed: metadataFiles.length
      }
    };

    if (this.options.debug) {
      console.log(`[TailwindExtractor] Merged ${metadataFiles.length} metadata files, found ${allClasses.size} unique classes`);
    }

    return {
      data: mergedData,
      fileCount: metadataFiles.length
    };
  }

  cleanup() {
    if (!this.tempDir) {
      return;
    }

    try {
      // Recursively remove the temp directory
      this.removeDirRecursive(this.tempDir);

      if (this.options.debug) {
        console.log(`[TailwindExtractor] Removed temp directory: ${this.tempDir}`);
      }

      this.tempDir = null;
      this.metadataFile = null;
    } catch (err) {
      console.warn(`[TailwindExtractor] Failed to cleanup temp directory: ${err.message}`);
    }
  }

  removeDirRecursive(dirPath) {
    if (fs.existsSync(dirPath)) {
      fs.readdirSync(dirPath).forEach((file) => {
        const curPath = path.join(dirPath, file);
        if (fs.lstatSync(curPath).isDirectory()) {
          this.removeDirRecursive(curPath);
        } else {
          fs.unlinkSync(curPath);
        }
      });
      fs.rmdirSync(dirPath);
    }
  }
}

module.exports = TailwindExtractorPlugin;
