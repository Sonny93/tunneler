import { defineConfig, presetIcons, presetWind4 } from 'unocss';

import { buildThemeColors, buildThemeCss } from './src/theme/theme_css';

const FRAMEWORK_SOURCE_PATTERN =
	/\.(vue|svelte|[jt]sx|vine.ts|mdx?|astro|elm|php|phtml|marko|html)($|\?)/;
const TYPESCRIPT_SOURCE_PATTERN = /\.ts($|\?)/;

export default defineConfig({
	presets: [presetWind4({ dark: 'class' }), presetIcons()],
	theme: { colors: buildThemeColors() },
	content: {
		pipeline: {
			include: [FRAMEWORK_SOURCE_PATTERN, TYPESCRIPT_SOURCE_PATTERN],
		},
	},
	preflights: [{ getCSS: buildThemeCss }],
});
