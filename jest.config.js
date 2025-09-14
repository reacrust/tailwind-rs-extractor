/**
 * Jest configuration for tailwind-extractor
 */

module.exports = {
  testEnvironment: 'node',
  roots: ['<rootDir>/tests'],
  testMatch: [
    '**/*.test.js',
    '**/*.spec.js'
  ],
  collectCoverageFrom: [
    'index.js',
    'lib/**/*.js'
  ],
  coveragePathIgnorePatterns: [
    '/node_modules/',
    '/tests/'
  ],
  coverageReporters: [
    'text',
    'lcov',
    'html'
  ],
  testTimeout: 30000, // 30 seconds for build operations
  verbose: true
};