import type { ThemeTokens } from './tokens';

const SHORT_NAMES: Readonly<Partial<Record<keyof ThemeTokens, string>>> = {
	background: 'bg',
	textMuted: 'muted',
};

function toKebabCase(camelCaseName: string): string {
	return camelCaseName.replaceAll(
		/[A-Z]/g,
		(letter) => `-${letter.toLowerCase()}`
	);
}

/** Name of a token without prefix: `surfaceRaised` becomes `surface-raised`, `textMuted` becomes `muted`. */
export function getTokenName(tokenKey: keyof ThemeTokens): string {
	return SHORT_NAMES[tokenKey] ?? toKebabCase(tokenKey);
}

export function getTokenCssVariable(tokenKey: keyof ThemeTokens): string {
	return `--tn-${getTokenName(tokenKey)}`;
}
