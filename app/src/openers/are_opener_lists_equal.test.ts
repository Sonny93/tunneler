import { describe, expect, it } from 'vitest';

import { areOpenerListsEqual } from '#/openers/are_opener_lists_equal';
import { buildOpenerSpec } from '#/testing/factories';

describe('areOpenerListsEqual', () => {
	it('should be true for copies of the same applications', () => {
		expect(areOpenerListsEqual([buildOpenerSpec()], [buildOpenerSpec()])).toBe(
			true
		);
	});

	it('should be false when an argument differs', () => {
		expect(
			areOpenerListsEqual(
				[buildOpenerSpec()],
				[buildOpenerSpec({ args: ['--other'] })]
			)
		).toBe(false);
	});

	it('should be false when a port differs', () => {
		expect(
			areOpenerListsEqual(
				[buildOpenerSpec()],
				[buildOpenerSpec({ ports: [1] })]
			)
		).toBe(false);
	});

	it('should be false when an application was added', () => {
		expect(areOpenerListsEqual([], [buildOpenerSpec()])).toBe(false);
	});

	it('should be false when the order differs', () => {
		const first = buildOpenerSpec({ id: 'a' });
		const second = buildOpenerSpec({ id: 'b' });

		expect(areOpenerListsEqual([first, second], [second, first])).toBe(false);
	});
});
