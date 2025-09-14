/**
 * Integration tests for TailwindExtractor HTML CSS injection functionality
 */

const path = require('path');
const fs = require('fs-extra');
const tmp = require('tmp');
const { rspack } = require('@rspack/core');
const HtmlRspackPlugin = require('html-rspack-plugin');
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

describe('TailwindExtractor HTML Injection', () => {
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

  test('should inject CSS link when injectCSS is "link"', async () => {
    const outputPath = path.join(tempDir.name, 'dist');
    const htmlTemplate = path.join(tempDir.name, 'template.html');

    // Create a simple HTML template
    await fs.writeFile(htmlTemplate, `<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>`);

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
        new HtmlRspackPlugin({
          template: htmlTemplate,
          filename: 'index.html',
        }),
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: 'link', // Test link injection
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
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

    // Read the generated HTML file
    const htmlPath = path.join(outputPath, 'index.html');
    const htmlExists = await fs.pathExists(htmlPath);
    expect(htmlExists).toBe(true);

    const htmlContent = await fs.readFile(htmlPath, 'utf-8');

    // Verify CSS link was injected
    expect(htmlContent).toMatch(/<link href="tailwind\.[a-f0-9]+\.css" rel="stylesheet">/);

    // Verify the CSS file was actually generated
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(1);

    // Verify script tags are also present (from HtmlRspackPlugin)
    expect(htmlContent).toContain('<script');
    expect(htmlContent).toContain('bundle.js');
  });

  test('should inject inline CSS when injectCSS is "inline"', async () => {
    const outputPath = path.join(tempDir.name, 'dist');
    const htmlTemplate = path.join(tempDir.name, 'template.html');

    // Create a simple HTML template
    await fs.writeFile(htmlTemplate, `<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>`);

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
        new HtmlRspackPlugin({
          template: htmlTemplate,
          filename: 'index.html',
        }),
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: 'inline', // Test inline injection
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
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

    // Read the generated HTML file
    const htmlPath = path.join(outputPath, 'index.html');
    const htmlExists = await fs.pathExists(htmlPath);
    expect(htmlExists).toBe(true);

    const htmlContent = await fs.readFile(htmlPath, 'utf-8');

    // Verify inline style tag was injected
    expect(htmlContent).toMatch(/<style data-source="tailwind-extractor">[\s\S]*<\/style>/);

    // Verify CSS contains expected classes
    expect(htmlContent).toContain('mx-auto');
    expect(htmlContent).toContain('bg-blue-600');

    // Verify NO external CSS file was generated for inline mode
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(0);

    // Verify script tags are also present (from HtmlRspackPlugin)
    expect(htmlContent).toContain('<script');
    expect(htmlContent).toContain('bundle.js');
  });

  test('should inject CSS with CSP nonce when configured', async () => {
    const outputPath = path.join(tempDir.name, 'dist');
    const htmlTemplate = path.join(tempDir.name, 'template.html');

    // Create HTML template with CSP nonce placeholder
    await fs.writeFile(htmlTemplate, `<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
    <meta http-equiv="Content-Security-Policy" content="style-src 'nonce-<!-- CSP_NONCE -->';">
</head>
<body>
    <div id="root"></div>
</body>
</html>`);

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
        new HtmlRspackPlugin({
          template: htmlTemplate,
          filename: 'index.html',
        }),
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: 'inline',
          csp: {
            nonce: '<!-- CSP_NONCE -->', // CSP nonce placeholder
          },
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
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

    // Read the generated HTML file
    const htmlPath = path.join(outputPath, 'index.html');
    const htmlContent = await fs.readFile(htmlPath, 'utf-8');

    // Verify inline style tag was injected with nonce
    expect(htmlContent).toMatch(/<style data-source="tailwind-extractor" nonce="<!-- CSP_NONCE -->">[\s\S]*<\/style>/);
  });

  test('should inject link with CSP nonce when configured', async () => {
    const outputPath = path.join(tempDir.name, 'dist');
    const htmlTemplate = path.join(tempDir.name, 'template.html');

    // Create HTML template
    await fs.writeFile(htmlTemplate, `<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>`);

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
        new HtmlRspackPlugin({
          template: htmlTemplate,
          filename: 'index.html',
        }),
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: 'link',
          csp: {
            nonce: '<!-- CSP_NONCE -->', // CSP nonce placeholder
          },
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
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

    // Read the generated HTML file
    const htmlPath = path.join(outputPath, 'index.html');
    const htmlContent = await fs.readFile(htmlPath, 'utf-8');

    // Verify CSS link was injected with nonce
    expect(htmlContent).toMatch(/<link href="tailwind\.[a-f0-9]+\.css" rel="stylesheet" nonce="<!-- CSP_NONCE -->">/);
  });

  test('should not inject CSS when injectCSS is false', async () => {
    const outputPath = path.join(tempDir.name, 'dist');
    const htmlTemplate = path.join(tempDir.name, 'template.html');

    // Create a simple HTML template
    await fs.writeFile(htmlTemplate, `<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>`);

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
        new HtmlRspackPlugin({
          template: htmlTemplate,
          filename: 'index.html',
        }),
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: false, // Disable injection
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
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

    // Read the generated HTML file
    const htmlPath = path.join(outputPath, 'index.html');
    const htmlContent = await fs.readFile(htmlPath, 'utf-8');

    // Verify NO CSS injection occurred
    expect(htmlContent).not.toMatch(/<link[^>]*tailwind[^>]*\.css/);
    expect(htmlContent).not.toMatch(/<style[^>]*data-source="tailwind-extractor"/);

    // But CSS file should still be generated
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(1);

    // Verify script tags are still present (from HtmlRspackPlugin)
    expect(htmlContent).toContain('<script');
    expect(htmlContent).toContain('bundle.js');
  });

  test('should work when HtmlRspackPlugin is not present', async () => {
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
        // NO HtmlRspackPlugin
        new TailwindExtractor({
          test: /\.jsx?$/,
          injectCSS: 'link', // This should gracefully fail
          css: {
            minify: false,
            noPreflight: false,
          },
          debug: false,
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      },
    };

    // Run the build
    const { stats } = await runBuild(config);

    // Check that build succeeded despite no HTML plugin
    expect(stats.hasErrors()).toBe(false);

    // CSS file should still be generated
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(1);
  });
});