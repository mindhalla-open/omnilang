module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/tests/**/*.test.ts'],
  coverageReporters: ['text-summary', 'json-summary', 'lcov'],
  // Ratchet floor, not a target. Measured 2026-08-06 at 56.96/44/66.31/56.55;
  // each threshold sits just below the measurement so an unrelated change
  // cannot silently erode coverage. Raise these when coverage rises — never
  // lower them to make a red build green.
  coverageThreshold: {
    global: {
      statements: 55,
      branches: 42,
      functions: 64,
      lines: 55,
    },
  },
};
