import { describe, expect, it } from 'vitest';

import { getThemeAriaLabel } from '#/header/theme_aria_label';

describe('getThemeAriaLabel', () => {
	it.each([
		['system', 'Thème : système, changer'],
		['light', 'Thème : clair, changer'],
		['dark', 'Thème : sombre, changer'],
	] as const)('should label the %s theme', (theme, label) => {
		expect(getThemeAriaLabel(theme)).toBe(label);
	});
});
