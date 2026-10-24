import { getTokenCssVariable, getTokenName } from './token_names';
import { THEME_TOKENS, type ThemeName, type ThemeTokens } from './tokens';

const DARK_MODE_SELECTOR = '.dark';

const GLOBAL_CSS = `
html, body { overflow: hidden; }
body { background: var(--tn-bg); color: var(--tn-text); }
:focus-visible { outline: 2px solid var(--tn-focus-ring); outline-offset: 2px; }
@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
@media (forced-colors: active) {
  .tn-status-badge { border: 1px solid CanvasText; }
  :focus-visible { outline-color: Highlight; }
  [class^='i-'], [class*=' i-'] { forced-color-adjust: none; }
}
`;

function getTokenKeys(): (keyof ThemeTokens)[] {
	return Object.keys(THEME_TOKENS.light) as (keyof ThemeTokens)[];
}

function buildVariableDeclarations(themeName: ThemeName): string {
	const tokens = THEME_TOKENS[themeName];
	return getTokenKeys()
		.map((tokenKey) => `${getTokenCssVariable(tokenKey)}: ${tokens[tokenKey]};`)
		.join(' ');
}

export function buildThemeCss(): string {
	const lightRule = `:root { color-scheme: light; ${buildVariableDeclarations('light')} }`;
	const darkRule = `${DARK_MODE_SELECTOR} { color-scheme: dark; ${buildVariableDeclarations('dark')} }`;
	return `${lightRule}\n${darkRule}\n${GLOBAL_CSS}`;
}

/** Maps `tn-<token>` color names to their CSS variables for UnoCSS `theme.colors`. */
export function buildThemeColors(): Record<string, string> {
	return Object.fromEntries(
		getTokenKeys().map((tokenKey) => [
			`tn-${getTokenName(tokenKey)}`,
			`var(${getTokenCssVariable(tokenKey)})`,
		])
	);
}
