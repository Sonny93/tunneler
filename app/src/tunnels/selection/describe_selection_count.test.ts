import { describe, expect, it } from 'vitest';

import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';

describe('describeSelectionCount', () => {
	it('should say that no tunnel is selected', () => {
		expect(describeSelectionCount(0)).toBe('Aucun tunnel sélectionné');
	});

	it('should use the singular for one tunnel', () => {
		expect(describeSelectionCount(1)).toBe('1 tunnel sélectionné');
	});

	it('should use the plural from two tunnels', () => {
		expect(describeSelectionCount(2)).toBe('2 tunnels sélectionnés');
	});
});
