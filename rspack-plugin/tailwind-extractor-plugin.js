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
    path.resolve(__dirname, '../../target/debug/tailwind-extractor-cli'),
    // Release build
    path.resolve(__dirname, '../../target/release/tailwind-extractor-cli'),
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
 */
class TailwindExtractorPlugin {
  constructor(options = {}) {
    this.options = {
      tempDir: options.tempDir,
      noPreflight: options.noPreflight || false,
      minify: options.minify || false,
      cleanupTempFiles: options.cleanupTempFiles !== false // Default true
    };
    this.cssFilename = null; // Store the generated CSS filename
  }
  
  apply(compiler) {
    const pluginName = 'TailwindExtractorPlugin';
    
    compiler.hooks.compilation.tap(pluginName, (compilation) => {
      // Get webpack/rspack sources utility
      const { sources } = compiler.webpack || require('@rspack/core');
      const { RawSource } = sources;
      
      // Hook into HtmlRspackPlugin to inject CSS link tag (if available)
      try {
        const HtmlRspackPlugin = require('html-rspack-plugin');
        if (HtmlRspackPlugin.getHooks) {
          const hooks = HtmlRspackPlugin.getHooks(compilation);
          
          // Inject CSS link tag into HTML head
          hooks.alterAssetTagGroups.tapAsync(pluginName, (data, callback) => {
            if (this.cssFilename) {
              // Create link tag for the CSS file
              data.headTags.push({
                tagName: 'link',
                attributes: {
                  rel: 'stylesheet',
                  href: `/dist/${this.cssFilename}`
                },
                voidTag: true
              });
            }
            callback(null, data);
          });
        }
      } catch (e) {
        // HtmlRspackPlugin not available, skip HTML injection
      }
      
      // Hook into processAssets to generate CSS after all modules are processed
      compilation.hooks.processAssets.tapAsync(
        {
          name: pluginName,
          stage: compilation.constructor.PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE
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
            
            // Generate filename with content hash for production, simple name for dev
            const isDev = compiler.options.mode === 'development';
            const filename = isDev ? 'tailwind.css' : `tailwind.${generateContentHash(css)}.css`;
            
            // Store filename for HTML injection
            this.cssFilename = filename;
            
            // Emit CSS asset
            compilation.emitAsset(filename, new RawSource(css));
            
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
  }
}

module.exports = TailwindExtractorPlugin;