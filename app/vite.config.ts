import UnoCSS from 'unocss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

const DEV_SERVER_PORT = 1420;
const isWindowsPlatform = process.env.TAURI_ENV_PLATFORM === 'windows';
const isDebugBuild = Boolean(process.env.TAURI_ENV_DEBUG);

export default defineConfig({
	plugins: [react(), UnoCSS()],
	clearScreen: false,
	envPrefix: ['VITE_', 'TAURI_ENV_*'],
	server: {
		port: DEV_SERVER_PORT,
		strictPort: true,
		watch: { ignored: ['**/src-tauri/**'] },
	},
	build: {
		target: isWindowsPlatform ? 'chrome105' : 'safari13',
		minify: isDebugBuild ? false : 'oxc',
		sourcemap: isDebugBuild,
	},
	test: {
		environment: 'jsdom',
		execArgv: ['--no-experimental-webstorage'],
		setupFiles: ['./src/test_setup.ts'],
	},
});
