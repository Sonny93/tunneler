import { minimalstuffPreset } from '@minimalstuff/tooling/oxc/lint';

const RELATIVE_IMPORT_PATTERN = {
	group: ['./*', '../*'],
	message: 'Use a subpath import (#folder/file.ts) instead of a relative path.',
};

export default minimalstuffPreset({
	react: true,
	lingui: { allowedStrings: ['Tunneler', '127.0.0.1'] },
	plugins: ['jsx-a11y'],
	rules: {
		// WebKit drops the list role of a `list-style: none` <ul>, so the explicit role is needed
		'jsx-a11y/no-redundant-roles': ['error', { ul: ['list'] }],
		// Replaces the preset's rule of the same name: keep its relative-import pattern here
		'no-restricted-imports': [
			'error',
			{
				paths: [
					{
						name: '@minimalstuff/ui',
						importNames: ['toast'],
						message:
							'Show toasts through pushToast (#/shared/toasts/push_toast.ts) instead of importing toast.',
					},
				],
				patterns: [RELATIVE_IMPORT_PATTERN],
			},
		],
	},
	overrides: [
		{
			files: [
				'src/shared/toasts/push_toast.ts',
				'src/testing/shown_toasts.ts',
				'src/test_setup.ts',
			],
			rules: {
				'no-restricted-imports': [
					'error',
					{ patterns: [RELATIVE_IMPORT_PATTERN] },
				],
			},
		},
	],
	ignorePatterns: ['src-tauri/**', 'dist/**'],
});
