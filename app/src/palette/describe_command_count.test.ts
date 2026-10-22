import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeCommandCount } from '#/palette/describe_command_count';

describe('describeCommandCount', () => {
	it.each([
		[0, 'No commands'],
		[1, '1 command'],
		[12, '12 commands'],
	])('should describe %i commands', (count, expected) => {
		expect(describeCommandCount(count, i18n)).toBe(expected);
	});
});
