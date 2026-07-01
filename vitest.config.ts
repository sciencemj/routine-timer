/// <reference types="vitest/config" />
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { fileURLToPath, URL } from 'node:url';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  resolve: { alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) } },
  test: { environment: 'jsdom', globals: true, setupFiles: ['./vitest-setup.ts'] },
});
