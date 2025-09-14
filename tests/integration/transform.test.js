/**
 * Tests to verify the transformer preserves JavaScript functionality
 */

const path = require('path');
const fs = require('fs-extra');
const tmp = require('tmp');
const { spawn } = require('child_process');

// Helper to create a temporary directory
function createTempDir() {
  return tmp.dirSync({ unsafeCleanup: true });
}

// Helper to run the CLI transformer
function transformCode(cliPath, source, metadataPath) {
  return new Promise((resolve, reject) => {
    const args = ['transform', metadataPath, '--source-file', 'test.jsx'];

    const child = spawn(cliPath, args, {
      stdio: ['pipe', 'pipe', 'pipe']
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', chunk => {
      stdout += chunk.toString();
    });

    child.stderr.on('data', chunk => {
      stderr += chunk.toString();
    });

    child.on('close', code => {
      if (code !== 0) {
        reject(new Error(`CLI exited with code ${code}: ${stderr}`));
      } else {
        resolve({
          transformedCode: stdout,
          stderr,
          metadataPath
        });
      }
    });

    child.stdin.write(source);
    child.stdin.end();
  });
}

describe('TailwindExtractor Transformation', () => {
  let tempDir;
  const cliPath = path.join(__dirname, '../../bins/x86_64-linux/tailwind-extractor-cli');

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    if (tempDir) {
      tempDir.removeCallback();
    }
  });

  test('preserves conditional class logic with ternary operator', async () => {
    const source = `
      const isActive = Math.random() > 0.5;
      const baseClass = 'px-4 py-2 rounded-lg';
      const stateClass = isActive ? 'bg-green-500 text-white' : 'bg-gray-200 text-gray-700';
    `;

    const metadataPath = path.join(tempDir.name, 'metadata.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check that the transformed code still contains conditional logic
    expect(transformedCode).toContain('isActive');
    expect(transformedCode).toContain('?');
    expect(transformedCode).toContain(':');

    // Check that metadata was written with real Tailwind classes
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('px-4');
    expect(metadata.classes).toContain('py-2');
    expect(metadata.classes).toContain('rounded-lg');
    expect(metadata.classes).toContain('bg-green-500');
    expect(metadata.classes).toContain('text-white');
    expect(metadata.classes).toContain('bg-gray-200');
    expect(metadata.classes).toContain('text-gray-700');
  });

  test('preserves array-based class concatenation', async () => {
    const source = `
      const classes = [
        'btn px-4 py-2',
        isEnabled && 'bg-blue-500 hover:bg-blue-600',
        size === 'large' && 'text-xl font-bold'
      ].filter(Boolean).join(' ');
    `;

    const metadataPath = path.join(tempDir.name, 'metadata2.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check that array structure is preserved
    expect(transformedCode).toContain('[');
    expect(transformedCode).toContain(']');
    expect(transformedCode).toContain('filter');
    expect(transformedCode).toContain('join');

    // Check metadata
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('btn');
    expect(metadata.classes).toContain('px-4');
    expect(metadata.classes).toContain('py-2');
    expect(metadata.classes).toContain('bg-blue-500');
    expect(metadata.classes).toContain('hover:bg-blue-600');
    expect(metadata.classes).toContain('text-xl');
    expect(metadata.classes).toContain('font-bold');
  });

  test('preserves JSX className attributes with static strings', async () => {
    const source = `
      import React from 'react';

      function Component({ isError }) {
        return (
          <div className={isError ? 'bg-red-100 border-red-500' : 'bg-green-100 border-green-500'}>
            <span className="text-sm font-medium px-2 py-1">Text</span>
          </div>
        );
      }
    `;

    const metadataPath = path.join(tempDir.name, 'metadata3.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check JSX structure is preserved
    expect(transformedCode).toContain('className');
    expect(transformedCode).toContain('isError');

    // Check metadata - only static strings are extracted
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('bg-red-100');
    expect(metadata.classes).toContain('border-red-500');
    expect(metadata.classes).toContain('bg-green-100');
    expect(metadata.classes).toContain('border-green-500');
    expect(metadata.classes).toContain('text-sm');
    expect(metadata.classes).toContain('font-medium');
    expect(metadata.classes).toContain('px-2');
    expect(metadata.classes).toContain('py-1');
  });

  test('handles object-based variant mapping', async () => {
    const source = `
      const variants = {
        primary: 'bg-blue-600 hover:bg-blue-700 text-white',
        secondary: 'bg-gray-600 hover:bg-gray-700 text-white',
        danger: 'bg-red-600 hover:bg-red-700 text-white'
      };

      const className = variants[variant] || 'bg-gray-400 text-gray-800';
    `;

    const metadataPath = path.join(tempDir.name, 'metadata4.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check object structure is preserved
    expect(transformedCode).toContain('variants');
    expect(transformedCode).toContain('||');

    // Check metadata
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('bg-blue-600');
    expect(metadata.classes).toContain('hover:bg-blue-700');
    expect(metadata.classes).toContain('bg-gray-600');
    expect(metadata.classes).toContain('hover:bg-gray-700');
    expect(metadata.classes).toContain('bg-red-600');
    expect(metadata.classes).toContain('hover:bg-red-700');
    expect(metadata.classes).toContain('text-white');
    expect(metadata.classes).toContain('bg-gray-400');
    expect(metadata.classes).toContain('text-gray-800');
  });

  test('preserves static classes in loops', async () => {
    const source = `
      const items = ['a', 'b', 'c'];
      const elements = items.map((item, idx) => ({
        className: 'px-3 py-2 border-b hover:bg-gray-50'
      }));
    `;

    const metadataPath = path.join(tempDir.name, 'metadata5.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check map structure is preserved
    expect(transformedCode).toContain('map');
    expect(transformedCode).toContain('className');

    // Check metadata - only static classes are extracted
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('px-3');
    expect(metadata.classes).toContain('py-2');
    expect(metadata.classes).toContain('border-b');
    expect(metadata.classes).toContain('hover:bg-gray-50');
  });

  test('handles spread operator with className override', async () => {
    const source = `
      const baseProps = { className: 'flex items-center gap-2' };
      const overrideProps = condition ? { className: 'inline-flex items-start gap-4' } : {};

      return <div {...baseProps} {...overrideProps} />;
    `;

    const metadataPath = path.join(tempDir.name, 'metadata6.json');
    const { transformedCode } = await transformCode(cliPath, source, metadataPath);

    // Check spread operator is preserved
    expect(transformedCode).toContain('...');
    expect(transformedCode).toContain('baseProps');
    expect(transformedCode).toContain('overrideProps');

    // Check metadata
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('flex');
    expect(metadata.classes).toContain('items-center');
    expect(metadata.classes).toContain('gap-2');
    expect(metadata.classes).toContain('inline-flex');
    expect(metadata.classes).toContain('items-start');
    expect(metadata.classes).toContain('gap-4');
  });

  test('extracts classes with obfuscation enabled', async () => {
    const source = `
      const className = "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2";
    `;

    const metadataPath = path.join(tempDir.name, 'metadata7.json');
    const args = ['transform', metadataPath, '--obfuscate', '--source-file', 'test.jsx'];

    const child = spawn(cliPath, args, {
      stdio: ['pipe', 'pipe', 'pipe']
    });

    let stdout = '';

    await new Promise((resolve, reject) => {
      child.stdout.on('data', chunk => {
        stdout += chunk.toString();
      });

      child.on('close', code => {
        if (code !== 0) {
          reject(new Error(`CLI exited with code ${code}`));
        } else {
          resolve();
        }
      });

      child.stdin.write(source);
      child.stdin.end();
    });

    // With obfuscation, the original class names might be transformed
    // But the structure should be preserved
    expect(stdout).toContain('className');
    expect(stdout).toContain('=');

    // Check metadata contains the original classes
    const metadata = await fs.readJson(metadataPath);
    expect(metadata.classes).toContain('bg-blue-500');
    expect(metadata.classes).toContain('hover:bg-blue-600');
    expect(metadata.classes).toContain('text-white');
    expect(metadata.classes).toContain('px-4');
    expect(metadata.classes).toContain('py-2');
  });
});