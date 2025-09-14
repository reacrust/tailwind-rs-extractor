/**
 * Tailwind Extractor Plugin for Rspack
 * 
 * Collects metadata files from the loader phase, merges extracted classes,
 * and generates final Tailwind CSS using the tailwind-extractor CLI.
 */

const { spawn } = require('node:child_process');
const { promises: fs } = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const os = require('node:os');

// Cache the CLI path discovery
let cliPath = null;

/**
 * Find the tailwind-extractor CLI binary
 * @returns {Promise<string>} Path to the CLI binary
 */
async function findCliPath() {
  if (cliPath) return cliPath;
  
  // Check common locations in order of preference
  const candidates = [
    // Debug build
    path.resolve(__dirname, '../../../target/debug/tailwind-extractor-cli'),
    // Release build
    path.resolve(__dirname, '../../../target/release/tailwind-extractor-cli'),
    // System-wide installation
    'tailwind-extractor-cli'
  ];
  
  for (const candidate of candidates) {
    try {
      await fs.access(candidate, fs.constants.X_OK);
      cliPath = candidate;
      return candidate;
    } catch {
      // Try next candidate
    }
  }
  
  throw new Error('tailwind-extractor CLI binary not found. Please build it first.');
}

/**
 * Execute CLI command with piped input
 * @param {string} cliPath - Path to CLI binary
 * @param {string[]} args - Command arguments
 * @param {string} input - Input to pipe to stdin
 * @returns {Promise<{stdout: string, stderr: string}>}
 */
function execCliCommand(cliPath, args, input) {
  return new Promise((resolve, reject) => {
    const child = spawn(cliPath, args, {
      stdio: ['pipe', 'pipe', 'pipe']
    });
    
    let stdout = '';
    let stderr = '';
    
    // Collect output
    child.stdout.on('data', chunk => {
      stdout += chunk.toString();
    });
    
    child.stderr.on('data', chunk => {
      stderr += chunk.toString();
    });
    
    // Handle completion
    child.on('close', code => {
      if (code !== 0) {
        reject(new Error(`CLI exited with code ${code}: ${stderr}`));
      } else {
        resolve({ stdout, stderr });
      }
    });
    
    child.on('error', err => {
      reject(new Error(`Failed to spawn CLI: ${err.message}`));
    });
    
    // Send input
    child.stdin.write(input);
    child.stdin.end();
  });
}

/**
 * Get temp directory for metadata files
 * @returns {Promise<string>} Path to temp directory
 */
async function getTempDir() {
  const tempBase = path.join(os.tmpdir(), 'tailwind-extractor');
  await fs.mkdir(tempBase, { recursive: true });
  return tempBase;
}

/**
 * Collect all metadata files from temp directory
 * @param {string} tempDir - Temp directory path
 * @returns {Promise<Array<Object>>} Array of metadata objects
 */
async function collectMetadataFiles(tempDir) {
  try {
    const files = await fs.readdir(tempDir);
    const metadataFiles = files.filter(f => f.endsWith('.json'));
    
    const metadataList = [];
    for (const file of metadataFiles) {
      try {
        const content = await fs.readFile(path.join(tempDir, file), 'utf8');
        const metadata = JSON.parse(content);
        metadataList.push(metadata);
      } catch (err) {
        // Skip invalid files
        console.warn(`Failed to read metadata file ${file}:`, err.message);
      }
    }
    
    return metadataList;
  } catch (err) {
    // Directory doesn't exist or not readable
    return [];
  }
}

/**
 * Merge classes from multiple metadata objects
 * @param {Array<Object>} metadataList - Array of metadata objects
 * @returns {Array<string>} Deduplicated array of classes
 */
function mergeClasses(metadataList) {
  const classSet = new Set();
  
  for (const metadata of metadataList) {
    if (metadata.classes && Array.isArray(metadata.classes)) {
      for (const className of metadata.classes) {
        classSet.add(className);
      }
    }
  }
  
  return Array.from(classSet);
}

/**
 * Generate content hash for CSS
 * @param {string} content - CSS content
 * @returns {string} 8-character hash
 */
function generateContentHash(content) {
  return crypto
    .createHash('sha256')
    .update(content)
    .digest('hex')
    .slice(0, 8);
}

/**
 * Tailwind Extractor Plugin
 * 
 * This plugin runs after the loader phase to collect all extracted Tailwind classes
 * and generate a single CSS file that gets properly injected by HtmlRspackPlugin.
 */
class TailwindExtractorPlugin {
  constructor(options = {}) {
    this.options = {
      tempDir: options.tempDir,
      noPreflight: options.noPreflight || false,
      minify: options.minify || false,
      cleanupTempFiles: options.cleanupTempFiles !== false, // Default true
      HtmlPlugin: options.HtmlPlugin // Pass in HtmlRspackPlugin reference
    };
    this.cssFilename = null; // Store the generated CSS filename
  }
  
  apply(compiler) {
    const pluginName = 'TailwindExtractorPlugin';
    const isDev = compiler.options.mode === 'development';
    
    // Store reference to self for inner scopes
    const self = this;
    
    // Hook into thisCompilation for proper timing
    compiler.hooks.thisCompilation.tap(pluginName, (compilation) => {
      // Get webpack/rspack sources utility
      const { sources } = compiler.webpack || require('@rspack/core');
      const { RawSource } = sources;
      
      // Track if we've generated CSS
      let cssGenerated = false;
      let cssFilename = null;
      
      // Process assets in ADDITIONAL stage - earlier than SUMMARIZE
      // This ensures CSS is available before HtmlRspackPlugin processes
      compilation.hooks.processAssets.tapAsync(
        {
          name: pluginName,
          // ADDITIONAL stage is for adding new assets - perfect for our CSS
          // This runs before HtmlRspackPlugin which typically runs at SUMMARIZE or later
          stage: compilation.constructor.PROCESS_ASSETS_STAGE_ADDITIONAL
        },
        async (assets, callback) => {
          try {
            // Find CLI binary
            const cliPath = await findCliPath();
            
            // Get temp directory
            const tempDir = this.options.tempDir || await getTempDir();
            
            // Collect all metadata files
            const metadataList = await collectMetadataFiles(tempDir);
            
            if (metadataList.length === 0) {
              // No metadata files found, skip CSS generation
              return callback();
            }
            
            // Merge all classes
            const allClasses = mergeClasses(metadataList);
            
            if (allClasses.length === 0) {
              // No classes found, skip CSS generation
              return callback();
            }
            
            // Create merged metadata for generate mode
            const mergedMetadata = {
              classes: allClasses,
              sourceFile: 'merged',
              processedAt: new Date().toISOString(),
              version: '1.0.0',
              stats: {
                originalCount: allClasses.length,
                uniqueCount: allClasses.length
              }
            };
            
            // Build CLI arguments for generate mode
            const args = ['generate'];
            
            if (this.options.noPreflight) {
              args.push('--no-preflight');
            }
            
            if (this.options.minify) {
              args.push('--minify');
            }
            
            // Execute generate command
            const { stdout: css, stderr } = await execCliCommand(
              cliPath,
              args,
              JSON.stringify(mergedMetadata)
            );
            
            // Log any warnings from stderr
            if (stderr && stderr.trim()) {
              console.warn(`tailwind-extractor generate: ${stderr}`);
            }
            
            // Skip if no CSS generated
            if (!css || !css.trim()) {
              return callback();
            }
            
            // Generate filename with content hash for production
            const filename = isDev ? 'tailwind.css' : `tailwind.${generateContentHash(css)}.css`;
            this.cssFilename = filename;
            cssFilename = filename;
            cssGenerated = true;
            
            // Emit CSS asset with proper metadata for HtmlRspackPlugin
            const source = new RawSource(css);
            compilation.emitAsset(filename, source, {
              // Mark as CSS asset type for proper detection
              minimized: this.options.minify,
              sourceFilename: filename,
              // This is crucial - tells HtmlRspackPlugin this is a CSS file
              // This matches what MiniCssExtractPlugin does
              hotModuleReplacement: false,
              // Mark this as a CSS content type
              contenthash: isDev ? undefined : generateContentHash(css)
            });
            
            // CRITICAL: Register CSS in compilation's assets for HtmlRspackPlugin
            // HtmlRspackPlugin looks for CSS files in different ways:
            // 1. Through entrypoint assets
            // 2. Through chunk files with .css extension
            // 3. Through compilation.assets with CSS metadata
            
            // Method 1: Add to all entrypoints as a CSS asset
            for (const [name, entrypoint] of compilation.entrypoints) {
              // Get the main chunk for this entrypoint first
              const mainChunk = entrypoint.getRuntimeChunk();
              if (mainChunk) {
                // Add CSS to the chunk's files
                mainChunk.files.add(filename);
                
                // Also add to auxiliary files for compatibility
                if (!mainChunk.auxiliaryFiles) {
                  mainChunk.auxiliaryFiles = new Set();
                }
                mainChunk.auxiliaryFiles.add(filename);
              }
              
              // Also add to all chunks in the entrypoint
              for (const chunk of entrypoint.chunks) {
                chunk.files.add(filename);
                if (!chunk.auxiliaryFiles) {
                  chunk.auxiliaryFiles = new Set();
                }
                chunk.auxiliaryFiles.add(filename);
              }
            }
            
            // Method 2: Add to initial chunks (what HtmlRspackPlugin processes)
            for (const chunk of compilation.chunks) {
              if (chunk.canBeInitial() || chunk.isOnlyInitial()) {
                chunk.files.add(filename);
                if (!chunk.auxiliaryFiles) {
                  chunk.auxiliaryFiles = new Set();
                }
                chunk.auxiliaryFiles.add(filename);
              }
            }
            
            // Cleanup temp files if requested
            if (this.options.cleanupTempFiles) {
              try {
                for (const metadata of metadataList) {
                  if (metadata.sourceFile) {
                    const basename = path.basename(metadata.sourceFile, path.extname(metadata.sourceFile));
                    const files = await fs.readdir(tempDir);
                    const toDelete = files.filter(f => f.startsWith(basename) && f.endsWith('.json'));
                    for (const file of toDelete) {
                      await fs.unlink(path.join(tempDir, file)).catch(() => {});
                    }
                  }
                }
              } catch {
                // Ignore cleanup errors
              }
            }
            
            callback();
          } catch (error) {
            callback(error);
          }
        }
      );
    });
    
    // Hook into HtmlRspackPlugin in the compilation phase  
    compiler.hooks.compilation.tap(pluginName, (compilation) => {
      // Use the passed HtmlPlugin reference if available
      const HtmlRspackPlugin = this.options.HtmlPlugin;
      
      // If HtmlRspackPlugin is available and supports hooks, use them
      if (HtmlRspackPlugin && HtmlRspackPlugin.getCompilationHooks) {
        try {
          const hooks = HtmlRspackPlugin.getCompilationHooks(compilation);
          
          // Hook into beforeAssetTagGeneration to add our CSS file
          hooks.beforeAssetTagGeneration.tapAsync(
            pluginName,
            (data, callback) => {
              // If we have a CSS file, ensure it's in the assets list
              if (self.cssFilename) {
                // Ensure css array exists
                if (!data.assets.css) {
                  data.assets.css = [];
                }
                
                // Add our CSS file if not already present
                if (!data.assets.css.includes(self.cssFilename)) {
                  // Get the publicPath from the compilation options
                  const publicPath = compilation.outputOptions.publicPath || '';
                  // Add the CSS file with the correct public path
                  const cssPath = publicPath + self.cssFilename;
                  data.assets.css.unshift(cssPath);
                }
              }
              
              callback(null, data);
            }
          );
        } catch (e) {
          // Silently continue if hooks fail
        }
      }
    });
  }
}

module.exports = TailwindExtractorPlugin;
