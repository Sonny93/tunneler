import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getThemeAriaLabel } from '#/header/theme_aria_label';

describe('getThemeAriaLabel', () => {
	it.each([
		['system', 'Theme: system, change'],
		['light', 'Theme: light, change'],
		['dark', 'Theme: dark, change'],
	] as const)('should label the %s theme', (theme, label) => {
		expect(getThemeAriaLabel(theme, i18n)).toBe(label);
	});
});
