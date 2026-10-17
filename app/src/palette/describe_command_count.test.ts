import { describe, expect, it } from 'vitest';

import { describeCommandCount } from '#/palette/describe_command_count';

describe('describeCommandCount', () => {
	it.each([
		[0, 'Aucune commande'],
		[1, '1 commande'],
		[12, '12 commandes'],
	])('should describe %i commands', (count, expected) => {
		expect(describeCommandCount(count)).toBe(expected);
	});
});
