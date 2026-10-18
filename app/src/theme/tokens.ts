export type ThemeTokens = {
	background: string;
	surface: string;
	surfaceRaised: string;
	/** Control boundary (input, switch, checkbox): at least 3:1 against background and surface. */
	border: string;
	/**
	 * Decorative separator (card outline, row dividers): below 3:1 on purpose.
	 * Never the only boundary of a control, only of regions the content already delimits.
	 */
	divider: string;
	/** Background of a selected row: text keeps 4.5:1 on it, the checkbox is the non-color cue. */
	selected: string;
	text: string;
	textMuted: string;
	/** Used for links and text: at least 4.5:1 against background. */
	accent: string;
	focusRing: string;
	success: string;
	successSubtle: string;
	warning: string;
	warningSubtle: string;
	danger: string;
	dangerSubtle: string;
	neutral: string;
	neutralSubtle: string;
	info: string;
	infoSubtle: string;
};

export type ThemeName = 'light' | 'dark';

export const THEME_TOKENS: Readonly<Record<ThemeName, ThemeTokens>> = {
	light: {
		background: '#ffffff',
		surface: '#f8f9fb',
		surfaceRaised: '#f0f2f6',
		border: '#6b7280',
		divider: '#e5e7eb',
		selected: '#e6edfc',
		text: '#111827',
		textMuted: '#4b5563',
		accent: '#1d4ed8',
		focusRing: '#2563eb',
		success: '#15803d',
		successSubtle: '#ecfdf3',
		warning: '#92400e',
		warningSubtle: '#fff7e6',
		danger: '#b91c1c',
		dangerSubtle: '#fef0f0',
		neutral: '#4b5563',
		neutralSubtle: '#f0f2f6',
		info: '#1d4ed8',
		infoSubtle: '#eef4ff',
	},
	dark: {
		background: '#0f1117',
		surface: '#161a23',
		surfaceRaised: '#1e2330',
		border: '#8b93a7',
		divider: '#262b38',
		selected: '#1b2740',
		text: '#e6e8ee',
		textMuted: '#a3abbd',
		accent: '#7aa7ff',
		focusRing: '#7aa7ff',
		success: '#4ade80',
		successSubtle: '#102a1b',
		warning: '#fbbf24',
		warningSubtle: '#2d2209',
		danger: '#ff8a8a',
		dangerSubtle: '#34161a',
		neutral: '#a3abbd',
		neutralSubtle: '#1e2330',
		info: '#7aa7ff',
		infoSubtle: '#15233f',
	},
};
