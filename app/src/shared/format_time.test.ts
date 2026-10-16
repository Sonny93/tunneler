import { describe, expect, it } from 'vitest';

import { formatTime } from '#/shared/format_time';

describe('formatTime', () => {
	it('should format local time as HH:MM:SS', () => {
		const localDate = new Date(2026, 0, 2, 3, 4, 5);

		expect(formatTime(localDate.getTime())).toBe('03:04:05');
	});
});
