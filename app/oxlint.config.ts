import { defineConfig } from 'oxlint';
import { minimalstuffPreset } from '@minimalstuff/tooling/oxc/lint';

export default defineConfig({
	extends: [minimalstuffPreset({ react: true })],
	plugins: ['jsx-a11y'],
	ignorePatterns: ['src-tauri/**', 'dist/**'],
});
