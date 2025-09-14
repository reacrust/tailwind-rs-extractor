/**
 * Runtime tests to verify processed JavaScript works correctly
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

describe('TailwindExtractor Runtime Execution', () => {
  let tempDir;

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    if (tempDir) {
      tempDir.removeCallback();
    }
  });

  test('processed JavaScript renders correctly with conditional classes', async () => {
    const outputPath = path.join(tempDir.name, 'dist');

    // Create RSpack configuration that outputs CommonJS for Node execution
    const config = {
      mode: 'development',
      entry: path.resolve(__dirname, 'fixtures', 'ConditionalClasses.jsx'),
      output: {
        path: outputPath,
        filename: 'bundle.js',
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
    const bundlePath = path.join(outputPath, 'bundle.js');
    const bundleExists = await fs.pathExists(bundlePath);
    expect(bundleExists).toBe(true);

    // Clear module cache to ensure fresh load
    delete require.cache[bundlePath];
    const ConditionalClasses = require(bundlePath).default;

    // Test 1: Render with default props
    const html1 = renderToString(React.createElement(ConditionalClasses));
    expect(html1).toContain('container');
    expect(html1).toContain('mx-auto');
    expect(html1).toContain('px-6');
    expect(html1).toContain('py-4');
    expect(html1).toContain('bg-gray-100'); // inactive state
    expect(html1).toContain('border-gray-300'); // inactive state
    expect(html1).toContain('rounded-md');
    expect(html1).toContain('font-[500]'); // font-medium is transformed to font-[500]
    expect(html1).toContain('bg-indigo-600'); // primary variant
    expect(html1).toContain('text-white');
    expect(html1).toContain('text-sm'); // medium size
    expect(html1).toContain('py-2');
    expect(html1).toContain('px-4');
    expect(html1).toContain('mt-6');
    expect(html1).toContain('p-4');
    expect(html1).toContain('rounded-lg');
    expect(html1).toContain('text-gray-600'); // inactive status
    expect(html1).toContain('font-[400]'); // font-normal is transformed to font-[400]
    expect(html1).toContain('inline-flex');
    expect(html1).toContain('bg-blue-100'); // default badge
    expect(html1).toContain('text-blue-800');
    expect(html1).toContain('All good'); // Error message check

    // Test 2: Render with isActive=true
    const html2 = renderToString(React.createElement(ConditionalClasses, { isActive: true }));
    expect(html2).toContain('bg-green-100'); // active state
    expect(html2).toContain('border-green-500'); // active state
    expect(html2).toContain('shadow-lg'); // active button
    expect(html2).toContain('transform');
    expect(html2).toContain('scale-110');
    expect(html2).toContain('text-green-700'); // active ok status
    expect(html2).toContain('font-[600]'); // font-semibold is transformed to font-[600]
    expect(html2).toContain('italic');
    expect(html2).not.toContain('bg-gray-100'); // not inactive
    expect(html2).not.toContain('border-gray-300');

    // Test 3: Render with hasError=true
    const html3 = renderToString(React.createElement(ConditionalClasses, { hasError: true }));
    expect(html3).toContain('ring-2');
    expect(html3).toContain('ring-red-400');
    expect(html3).toContain('ring-offset-2');
    expect(html3).toContain('bg-red-50'); // error state
    expect(html3).toContain('border-red-400');
    expect(html3).toContain('bg-red-100'); // error badge
    expect(html3).toContain('text-red-800');
    expect(html3).not.toContain('bg-blue-100'); // Should be overridden
    expect(html3).toContain('Error occurred'); // Error message check

    // Test 4: Render with isActive=true AND hasError=true
    const html4 = renderToString(React.createElement(ConditionalClasses, {
      isActive: true,
      hasError: true
    }));
    expect(html4).toContain('text-red-700'); // active error
    expect(html4).toContain('font-[700]'); // font-bold is transformed to font-[700]
    expect(html4).toContain('uppercase');
    expect(html4).not.toContain('text-green-700'); // not active ok
    expect(html4).not.toContain('text-gray-600'); // not inactive

    // Test 5: Render with different variant
    const html5 = renderToString(React.createElement(ConditionalClasses, { variant: 'danger' }));
    expect(html5).toContain('bg-red-600');
    expect(html5).toContain('hover:bg-red-700');
    expect(html5).not.toContain('bg-indigo-600'); // not primary

    // Test 6: Render with different size
    const html6 = renderToString(React.createElement(ConditionalClasses, { size: 'large' }));
    expect(html6).toContain('text-lg');
    expect(html6).toContain('py-3');
    expect(html6).toContain('px-6');
    expect(html6).not.toContain('text-sm'); // not medium

    // Test 7: Render with items array
    const html7 = renderToString(React.createElement(ConditionalClasses, {
      items: ['Item 1', 'Item 2', 'Item 3']
    }));
    expect(html7).toContain('divide-y');
    expect(html7).toContain('divide-gray-200');
    expect(html7).toContain('my-8');
    expect(html7).toContain('py-3');
    expect(html7).toContain('hover:bg-gray-100');

    // Load and verify the manifest contains all unique classes
    const manifestPath = path.join(outputPath, 'tailwind.manifest.json');
    const manifest = await fs.readJson(manifestPath);

    // Check that all our Tailwind classes are in the manifest
    const expectedClasses = [
      'container',
      'mx-auto',
      'px-6',
      'py-4',
      'bg-green-100',
      'border-green-500',
      'bg-gray-100',
      'border-gray-300',
      'bg-indigo-600',
      'hover:bg-indigo-700',
      'text-white',
      'bg-gray-600',
      'hover:bg-gray-700',
      'bg-red-600',
      'hover:bg-red-700',
      'text-xs',
      'py-1.5',
      'px-3',
      'text-sm',
      'py-2',
      'px-4',
      'text-lg',
      'py-3',
      'rounded-md',
      'font-medium',
      'transition-all',
      'duration-200',
      'ring-2',
      'ring-red-400',
      'ring-offset-2',
      'shadow-lg',
      'transform',
      'scale-110',
      'mt-6',
      'p-4',
      'rounded-lg',
      'bg-red-50',
      'border',
      'border-red-400',
      'divide-y',
      'divide-gray-200',
      'my-8',
      'hover:bg-gray-100',
      'text-red-700',
      'font-bold',
      'uppercase',
      'text-green-700',
      'font-semibold',
      'italic',
      'text-gray-600',
      'font-normal',
      'inline-flex',
      'bg-blue-100',
      'text-blue-800',
      'rounded-full',
      'bg-red-100',
      'text-red-800'
    ];

    expectedClasses.forEach(className => {
      expect(manifest.classes).toContain(className);
    });

    // Verify CSS was generated
    const cssFiles = await fs.readdir(outputPath)
      .then(files => files.filter(f => f.startsWith('tailwind.') && f.endsWith('.css')));
    expect(cssFiles).toHaveLength(1);

    // Read the CSS and verify it contains our unique classes
    const cssContent = await fs.readFile(
      path.join(outputPath, cssFiles[0]),
      'utf-8'
    );

    // Check a few key classes are in the CSS
    expect(cssContent).toContain('container');
    expect(cssContent).toContain('mx-auto');
    expect(cssContent).toContain('bg-indigo-600');
    expect(cssContent).toContain('rounded-md');
  });
});