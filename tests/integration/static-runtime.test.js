/**
 * Runtime tests to verify processed JavaScript with static Tailwind classes works correctly
 */

const path = require('path');
const fs = require('fs-extra');
const tmp = require('tmp');
const { rspack } = require('@rspack/core');
const TailwindExtractor = require('../../index');
const React = require('react');
const { renderToString } = require('react-dom/server');

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

describe('TailwindExtractor Static Runtime', () => {
  let tempDir;

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    if (tempDir) {
      tempDir.removeCallback();
    }
  });

  test('processed JavaScript with static Tailwind classes renders correctly', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    // Create RSpack configuration that outputs CommonJS for Node execution
    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'StaticClasses.jsx'),
      output: {
        path: outputPath,
        filename: 'static-bundle.js',
        library: {
          type: 'commonjs2'
        }
      },
      target: 'node', // Build for Node.js environment
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
                      runtime: 'classic', // Use classic for easier server-side testing
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
            noPreflight: true,
          },
          manifestFilename: 'tailwind.manifest.json',
          debug: false,
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      }
    };

    // Run the build
    const { stats } = await runBuild(config);
    expect(stats.hasErrors()).toBe(false);

    // Load the processed bundle
    const bundlePath = path.join(outputPath, 'static-bundle.js');
    const bundleExists = await fs.pathExists(bundlePath);
    expect(bundleExists).toBe(true);

    // Clear module cache to ensure fresh load
    delete require.cache[bundlePath];

    // Require React in the global scope for the bundle
    global.React = React;

    const StaticClasses = require(bundlePath).default;

    // Test rendering with default props
    const html1 = renderToString(React.createElement(StaticClasses));

    // Verify the HTML contains our Tailwind classes (some are transformed)
    expect(html1).toContain('container-2xl');
    expect(html1).toContain('px-9');
    expect(html1).toContain('border-amber-900');
    expect(html1).toContain('bg-indigo-950');
    expect(html1).toContain('text-zinc-50');
    expect(html1).toContain('text-7xl');
    expect(html1).toContain('font-[900]'); // font-black is transformed to font-[900]
    expect(html1).toContain('flex-row-reverse');
    expect(html1).toContain('gap-[1.75rem]'); // gap-7 is transformed to gap-[1.75rem]
    expect(html1).toContain('shadow-amber-500/50');
    expect(html1).toContain('backdrop-blur-3xl');
    expect(html1).toContain('Default Title'); // Check content renders

    // Test with custom title
    const html2 = renderToString(React.createElement(StaticClasses, { title: 'Custom Title' }));
    expect(html2).toContain('Custom Title');

    // Load and verify the manifest
    const manifestPath = path.join(outputPath, 'tailwind.manifest.json');
    const manifest = await fs.readJson(manifestPath);

    // Check that manifest contains the original Tailwind classes
    // Note: The manifest contains the original classes before transformation
    expect(manifest.classes).toContain('container-2xl');
    expect(manifest.classes).toContain('px-9');
    expect(manifest.classes).toContain('border-amber-900');
    expect(manifest.classes).toContain('bg-indigo-950');
    expect(manifest.classes).toContain('text-zinc-50');
    expect(manifest.classes).toContain('text-7xl');
    expect(manifest.classes).toContain('font-black'); // Original class in manifest
    expect(manifest.classes).toContain('tracking-tighter');
    expect(manifest.classes).toContain('flex-row-reverse');
    expect(manifest.classes).toContain('gap-7'); // Original class in manifest
    expect(manifest.classes).toContain('min-h-[50vh]');
    expect(manifest.classes).toContain('shadow-amber-500/50');
    expect(manifest.classes).toContain('backdrop-blur-3xl');
    expect(manifest.classes).toContain('decoration-wavy');
    expect(manifest.classes).toContain('underline-offset-8');
    expect(manifest.classes).toContain('prose-zinc');
    expect(manifest.classes).toContain('line-clamp-6');
    expect(manifest.classes).toContain('text-[13px]');
    expect(manifest.classes).toContain('leading-[1.8]');
    expect(manifest.classes).toContain('font-[450]');

    // Verify CSS was generated
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(1);

    // Read the CSS and verify it contains the classes
    const cssContent = await fs.readFile(
      path.join(outputPath, cssFiles[0]),
      'utf-8'
    );

    // Check a few key classes are in the CSS
    // Note: container-2xl may not generate CSS if not recognized by Tailwind
    expect(cssContent).toContain('.px-9');
    expect(cssContent).toContain('.bg-indigo-950');
    expect(cssContent).toContain('.text-7xl');

    // Clean up global
    delete global.React;
  });

  test('conditional Tailwind classes are extracted and render works', async () => {
    const outputPath = path.join(tempDir.name, 'dist2');

    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'ConditionalTailwind.jsx'),
      output: {
        path: outputPath,
        filename: 'conditional-bundle.js',
        library: {
          type: 'commonjs2'
        }
      },
      target: 'node',
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
                      runtime: 'classic',
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
          manifestFilename: 'tailwind.manifest.json',
        }),
      ],
      resolve: {
        extensions: ['.js', '.jsx'],
      }
    };

    // Run the build
    const { stats } = await runBuild(config);
    expect(stats.hasErrors()).toBe(false);

    // Load the bundle
    const bundlePath = path.join(outputPath, 'conditional-bundle.js');
    delete require.cache[bundlePath];
    global.React = React;

    const ConditionalTailwind = require(bundlePath).default;

    // Test rendering with different prop combinations
    const html1 = renderToString(React.createElement(ConditionalTailwind));
    expect(html1).toContain('mx-auto');
    expect(html1).toContain('px-6');

    const html2 = renderToString(React.createElement(ConditionalTailwind, { isActive: true }));
    expect(html2).toContain('mx-auto');

    const html3 = renderToString(React.createElement(ConditionalTailwind, { variant: 'danger' }));
    expect(html3).toContain('rounded-lg');

    // Load manifest and check static strings are extracted
    const manifestPath = path.join(outputPath, 'tailwind.manifest.json');
    const manifest = await fs.readJson(manifestPath);

    // These static strings should definitely be extracted
    expect(manifest.classes).toContain('mx-auto');
    expect(manifest.classes).toContain('px-6');
    expect(manifest.classes).toContain('rounded-lg');
    expect(manifest.classes).toContain('font-semibold');
    expect(manifest.classes).toContain('transition-colors');
    expect(manifest.classes).toContain('mt-4');
    expect(manifest.classes).toContain('p-3');
    expect(manifest.classes).toContain('rounded');
    expect(manifest.classes).toContain('space-y-2');
    expect(manifest.classes).toContain('my-6');
    expect(manifest.classes).toContain('px-3');
    expect(manifest.classes).toContain('py-1');

    // Clean up
    delete global.React;
  });
});