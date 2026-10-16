import { defineConfig, presetIcons, presetWind4 } from 'unocss';

import { buildThemeColors, buildThemeCss } from './src/theme/theme_css';

export default defineConfig({
	presets: [presetWind4({ dark: 'class' }), presetIcons()],
	theme: { colors: buildThemeColors() },
	preflights: [{ getCSS: buildThemeCss }],
});
