import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';

describe('describeSelectionCount', () => {
	it('should say that no tunnel is selected', () => {
		expect(describeSelectionCount(0, i18n)).toBe('No tunnels selected');
	});

	it('should use the singular for one tunnel', () => {
		expect(describeSelectionCount(1, i18n)).toBe('1 tunnel selected');
	});

	it('should use the plural from two tunnels', () => {
		expect(describeSelectionCount(2, i18n)).toBe('2 tunnels selected');
	});
});
