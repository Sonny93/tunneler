import { describe, expect, it } from 'vitest';

import { describeOptionCount } from '#/host_config/describe_option_count';

describe('describeOptionCount', () => {
	it('should say none when empty', () => {
		expect(describeOptionCount(0)).toBe('Aucune option affichée');
	});

	it('should use the singular for one option', () => {
		expect(describeOptionCount(1)).toBe('1 option affichée');
	});

	it('should use the plural for several options', () => {
		expect(describeOptionCount(12)).toBe('12 options affichées');
	});
});
