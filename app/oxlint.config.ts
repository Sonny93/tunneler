import { minimalstuffPreset } from '@minimalstuff/tooling/oxc/lint';

export default minimalstuffPreset({
	react: true,
	plugins: ['jsx-a11y'],
	// WebKit drops the list role of a `list-style: none` <ul>, so the explicit role is needed
	rules: {
		'jsx-a11y/no-redundant-roles': ['error', { ul: ['list'] }],
	},
	ignorePatterns: ['src-tauri/**', 'dist/**'],
});
