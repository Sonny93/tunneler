import { describe, expect, it } from 'vitest';

import { formatActiveCount } from '#/tunnels/format_active_count';
import { describeActiveCount } from '#/tunnels/describe_active_count';

describe('formatActiveCount', () => {
	it('should use a slash', () => {
		expect(formatActiveCount({ running: 2, total: 5 })).toBe('2/5 actifs');
	});
});

describe('describeActiveCount', () => {
	it('should use sur', () => {
		expect(describeActiveCount({ running: 2, total: 5 })).toBe(
			'2 sur 5 actifs'
		);
	});
});
