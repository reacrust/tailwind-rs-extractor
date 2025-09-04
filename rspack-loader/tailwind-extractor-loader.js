/**
 * Tailwind Extractor Loader for Rspack
 * 
 * Transforms JavaScript/TypeScript files through the tailwind-extractor CLI,
 * extracting Tailwind classes and replacing them with traced versions.
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
 * Get or create temp directory for metadata files
 * @param {string} customTempDir - Custom temp directory path
 * @returns {Promise<string>} Path to temp directory
 */
async function getTempDir(customTempDir) {
  const tempBase = customTempDir || path.join(os.tmpdir(), 'tailwind-extractor');
  await fs.mkdir(tempBase, { recursive: true });
  return tempBase;
}

/**
 * Generate unique metadata filename based on source content
 * @param {string} source - Source code
 * @param {string} resourcePath - File path
 * @returns {string} Unique filename
 */
function generateMetadataFilename(source, resourcePath) {
  const hash = crypto
    .createHash('sha256')
    .update(source)
    .update(resourcePath)
    .digest('hex')
    .slice(0, 16);
  
  const basename = path.basename(resourcePath, path.extname(resourcePath));
  return `${basename}-${hash}.json`;
}

/**
 * Tailwind Extractor Loader
 * @this {import('webpack').LoaderContext}
 * @param {string} source - Source code
 * @param {any} sourceMap - Source map (if available)
 * @returns {Promise<void>}
 */
async function tailwindExtractorLoader(source, sourceMap) {
  const callback = this.async();
  
  try {
    // Get loader options
    const options = this.getOptions() || {};
    const {
      obfuscate = false,
      enabled = true,
      tempDir: customTempDir,
      skipMetadata = false
    } = options;
    
    // Skip processing if disabled
    if (!enabled) {
      return callback(null, source, sourceMap);
    }
    
    // Find CLI binary
    const cliPath = await findCliPath();
    
    // Build CLI arguments
    const args = ['transform'];
    
    // Only add metadata path if not skipping metadata generation
    let metadataPath = null;
    if (!skipMetadata) {
      // Prepare metadata output path
      const tempDir = await getTempDir(customTempDir);
      const metadataFilename = generateMetadataFilename(source, this.resourcePath);
      metadataPath = path.join(tempDir, metadataFilename);
      args.push(metadataPath);
    } else {
      // Use '-' to indicate no metadata output
      args.push('-');
    }
    
    if (obfuscate) {
      args.push('--obfuscate');
    }
    
    // Add source file info for better metadata (even if not emitting metadata, useful for debugging)
    args.push('--source-file', this.resourcePath);
    
    // Execute transformation
    const { stdout: transformedCode, stderr } = await execCliCommand(
      cliPath,
      args,
      source
    );
    
    // Log any warnings/info from stderr
    if (stderr && stderr.trim()) {
      this.emitWarning(new Error(`tailwind-extractor: ${stderr}`));
    }
    
    // Mark metadata file as dependency if it was created
    if (metadataPath) {
      this.addDependency(metadataPath);
    }
    
    // Return transformed code
    callback(null, transformedCode, sourceMap);
    
  } catch (error) {
    // Log error and pass through original source
    this.emitError(error);
    callback(null, source, sourceMap);
  }
}

module.exports = tailwindExtractorLoader;
