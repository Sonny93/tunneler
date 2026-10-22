import { defineConfig } from '@lingui/cli';

export default defineConfig({
	sourceLocale: 'en',
	locales: ['en', 'fr'],
	catalogs: [
		{
			path: '<rootDir>/src/locales/{locale}/messages',
			include: ['<rootDir>/src'],
			exclude: ['**/*.test.ts', '**/*.test.tsx', '**/testing/**'],
		},
	],
});
