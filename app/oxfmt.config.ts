import { minimalstuffPreset } from '@minimalstuff/tooling/oxc/fmt';

export default minimalstuffPreset({
	ignorePatterns: [
		'pnpm-lock.yaml',
		'dist/**',
		'src-tauri/target/**',
		'src-tauri/gen/**',
	],
});
