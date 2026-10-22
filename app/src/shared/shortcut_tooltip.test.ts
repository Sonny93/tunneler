import { describe, expect, it } from 'vitest';

import { formatShortcutTooltip } from '#/shared/shortcut_tooltip';

describe('formatShortcutTooltip', () => {
	it('should append the shortcut in parentheses after the label', () => {
		expect(formatShortcutTooltip('Settings', ',')).toBe('Settings (,)');
	});
});
