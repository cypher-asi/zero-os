import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@desktop': resolve(__dirname, 'src/desktop'),
      '@apps': resolve(__dirname, 'src/apps'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./test/setup.ts'],
    include: ['**/*.test.{ts,tsx}', '**/__tests__/**/*.test.{ts,tsx}'],
    exclude: ['node_modules', 'dist', 'pkg'],
    coverage: {
      reporter: ['text', 'html'],
      exclude: ['node_modules', 'test/**', 'pkg/**'],
    },
  },
});
