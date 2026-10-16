import { describe, expect, it } from 'vitest';

import { getContrastRatio } from './contrast_ratio';
import { THEME_TOKENS, type ThemeName, type ThemeTokens } from './tokens';

const TEXT_MINIMUM = 4.5;
const COMPONENT_MINIMUM = 3;
const THEME_NAMES: readonly ThemeName[] = ['light', 'dark'];
const TEXT_KEYS = ['text', 'textMuted'] as const;
const SURFACE_KEYS = ['background', 'surface', 'surfaceRaised'] as const;
const STATUSES = ['success', 'warning', 'danger', 'neutral', 'info'] as const;

const contrast = (
	tokens: Readonly<ThemeTokens>,
	foreground: keyof ThemeTokens,
	background: keyof ThemeTokens
) => getContrastRatio(tokens[foreground], tokens[background]);

describe.each(THEME_NAMES)('%s theme tokens', (themeName) => {
	const tokens = THEME_TOKENS[themeName];

	describe.each(TEXT_KEYS)('%s', (textKey) => {
		it.each(SURFACE_KEYS)('should reach 4.5:1 on %s', (surfaceKey) => {
			expect(contrast(tokens, textKey, surfaceKey)).toBeGreaterThanOrEqual(
				TEXT_MINIMUM
			);
		});
	});

	describe.each(STATUSES)('%s status', (status) => {
		it('should reach 4.5:1 as text on the background', () => {
			expect(contrast(tokens, status, 'background')).toBeGreaterThanOrEqual(
				TEXT_MINIMUM
			);
		});

		it('should reach 4.5:1 as text on its own subtle background', () => {
			expect(
				contrast(tokens, status, `${status}Subtle`)
			).toBeGreaterThanOrEqual(TEXT_MINIMUM);
		});
	});

	describe.each(['border', 'focusRing'] as const)('%s', (boundaryKey) => {
		it.each(['background', 'surface'] as const)(
			'should reach 3:1 against %s',
			(surfaceKey) => {
				expect(
					contrast(tokens, boundaryKey, surfaceKey)
				).toBeGreaterThanOrEqual(COMPONENT_MINIMUM);
			}
		);
	});

	it('should reach 4.5:1 for the accent on the background', () => {
		expect(contrast(tokens, 'accent', 'background')).toBeGreaterThanOrEqual(
			TEXT_MINIMUM
		);
	});
});
