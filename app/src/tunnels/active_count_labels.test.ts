import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeActiveCount } from '#/tunnels/describe_active_count';
import { formatActiveCount } from '#/tunnels/format_active_count';

describe('formatActiveCount', () => {
	it('should use a slash', () => {
		expect(formatActiveCount({ running: 2, total: 5 }, i18n)).toBe(
			'2/5 active'
		);
	});
});

describe('describeActiveCount', () => {
	it('should use of', () => {
		expect(describeActiveCount({ running: 2, total: 5 }, i18n)).toBe(
			'2 of 5 active'
		);
	});
});
