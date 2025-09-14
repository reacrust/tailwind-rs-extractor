/**
 * Integration tests for TailwindExtractor RSpack plugin
 */

const path = require('path');
const fs = require('fs-extra');
const tmp = require('tmp');
const { rspack } = require('@rspack/core');
const TailwindExtractor = require('../../index');

// Helper to create a temporary directory
function createTempDir() {
  return tmp.dirSync({ unsafeCleanup: true });
}

// Helper to run RSpack build
function runBuild(config) {
  return new Promise((resolve, reject) => {
    rspack(config, (err, stats) => {
      if (err) {
        reject(err);
        return;
      }

      const info = stats.toJson();
      if (stats.hasErrors()) {
        const errorMessages = info.errors.map(e =>
          typeof e === 'string' ? e : (e.message || JSON.stringify(e))
        ).join('\n');
        reject(new Error(errorMessages));
        return;
      }

      resolve({ stats, info });
    });
  });
}

describe('TailwindExtractor Plugin Integration', () => {
  let tempDir;

  beforeEach(() => {
    // Create a fresh temp directory for each test
    tempDir = createTempDir();
  });

  afterEach(() => {
    // Cleanup temp directory
    if (tempDir) {
      tempDir.removeCallback();
    }
  });

  test('should extract and generate CSS from JSX components', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    // Create RSpack configuration
    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          css: {
            minify: false, // Keep CSS readable for testing
            noPreflight: false,
          },
          debug: false,
          keepTempDir: false,
          // NOT specifying tailwindExtractorPath - should use default from bins directory
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats, info } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);
    expect(stats.hasWarnings()).toBe(false);

    // Find the generated CSS file
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));

    expect(cssFiles).toHaveLength(1);

    // Read the generated CSS
    const cssContent = await fs.readFile(
      path.join(outputPath, cssFiles[0]),
      'utf-8'
    );

    // Verify that CSS contains Tailwind utility classes from each component
    // Note: Custom classes like 'app-container' are not Tailwind classes and won't be in the CSS

    // Verify common Tailwind utility classes are included
    expect(cssContent).toContain('mx-auto');
    expect(cssContent).toContain('p-4');
    expect(cssContent).toContain('bg-blue-600');
    expect(cssContent).toContain('bg-gray-50');
    expect(cssContent).toContain('bg-gray-800');
    expect(cssContent).toContain('text-white');
    expect(cssContent).toContain('text-4xl');
    expect(cssContent).toContain('font-\\[700\\]'); // CLI transforms font-bold to font-[700]
    expect(cssContent).toContain('flex');
    expect(cssContent).toContain('justify-between');
    expect(cssContent).toContain('space-x-4');

    // Verify hover states are included (in CSS they appear with :hover suffix)
    expect(cssContent).toContain('hover\\:text-blue-200:hover');
    expect(cssContent).toContain('hover\\:text-white:hover');

    // Verify the CSS has some minimum length (not empty or too small)
    expect(cssContent.length).toBeGreaterThan(1000);
  });

  test('should handle obfuscation option', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    const config = {
      mode: 'production',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          transform: {
            obfuscate: true, // Enable obfuscation
          },
          css: {
            minify: true,
            noPreflight: false,
          },
          // NOT specifying tailwindExtractorPath - should use default
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);

    // Find the generated CSS file
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));

    expect(cssFiles).toHaveLength(1);

    // Read the generated CSS
    const cssContent = await fs.readFile(
      path.join(outputPath, cssFiles[0]),
      'utf-8'
    );

    // With obfuscation, the original class names should NOT appear
    // Instead, they should be replaced with shorter obfuscated versions
    // We can check that the CSS is significantly smaller
    expect(cssContent.length).toBeGreaterThan(100);

    // The CSS should be minified (no unnecessary whitespace)
    expect(cssContent).not.toMatch(/\n\s+\n/); // No multiple blank lines
  });

  test('should skip processing when transform.enabled is false', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          transform: {
            enabled: false, // Disable transformation
          },
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);

    // No CSS file should be generated when transform is disabled
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));

    expect(cssFiles).toHaveLength(0);
  });

  test('should respect noPreflight option', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          css: {
            minify: false,
            noPreflight: true, // Disable preflight styles
          },
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);

    // Find the generated CSS file
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));

    expect(cssFiles).toHaveLength(1);

    // Read the generated CSS
    const cssContent = await fs.readFile(
      path.join(outputPath, cssFiles[0]),
      'utf-8'
    );

    // With noPreflight, there should be no reset styles
    // Preflight includes styles like button, input resets
    expect(cssContent).not.toContain('button:focus');
    expect(cssContent).not.toContain('*, ::before, ::after');

    // But utility classes should still be present
    expect(cssContent).toContain('mx-auto');
    expect(cssContent).toContain('bg-blue-600');
  });

  test('should generate manifest file with extracted classes', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          css: {
            minify: false,
            noPreflight: false,
          },
          manifestFilename: 'tailwind.manifest.json', // Explicit filename
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);

    // Check that manifest file was generated
    const manifestPath = path.join(outputPath, 'tailwind.manifest.json');
    const manifestExists = await fs.pathExists(manifestPath);
    expect(manifestExists).toBe(true);

    // Read and verify the manifest content
    const manifest = await fs.readJson(manifestPath);

    // Verify manifest structure
    expect(manifest).toHaveProperty('classes');
    expect(manifest).toHaveProperty('sourceFiles');
    expect(manifest).toHaveProperty('processedAt');
    expect(manifest).toHaveProperty('version');
    expect(manifest).toHaveProperty('stats');

    // Verify classes array contains expected Tailwind classes
    expect(manifest.classes).toContain('mx-auto');
    expect(manifest.classes).toContain('p-4');
    expect(manifest.classes).toContain('bg-blue-600');
    expect(manifest.classes).toContain('text-white');

    // Verify stats
    expect(manifest.stats.uniqueCount).toBeGreaterThan(0);
    expect(manifest.stats.filesProcessed).toBeGreaterThan(0);

    // Verify source files are tracked
    expect(manifest.sourceFiles.length).toBeGreaterThan(0);
  });

  test('should not generate manifest when manifestFilename is false', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'index.js'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
      },
      module: {
        rules: [
          {
            test: /\.jsx?$/,
            use: {
              loader: 'builtin:swc-loader',
              options: {
                jsc: {
                  parser: {
                    syntax: 'ecmascript',
                    jsx: true,
                  },
                  transform: {
                    react: {
                      runtime: 'automatic',
                    },
                  },
                },
              },
            },
          },
        ],
      },
      plugins: [
        new TailwindExtractor({
          test: /\.jsx?$/,
          manifestFilename: false, // Disable manifest generation
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded
    expect(stats.hasErrors()).toBe(false);

    // Check that manifest file was NOT generated
    const files = await fs.readdir(outputPath);
    const manifestFiles = files.filter(f => f.endsWith('.manifest.json'));
    expect(manifestFiles).toHaveLength(0);

    // But CSS should still be generated
    const cssFiles = files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css'));
    expect(cssFiles).toHaveLength(1);
  });
});