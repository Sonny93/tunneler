import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeOptionCount } from '#/host_config/describe_option_count';

describe('describeOptionCount', () => {
	it('should say none when empty', () => {
		expect(describeOptionCount(0, i18n)).toBe('No options shown');
	});

	it('should use the singular for one option', () => {
		expect(describeOptionCount(1, i18n)).toBe('1 option shown');
	});

	it('should use the plural for several options', () => {
		expect(describeOptionCount(12, i18n)).toBe('12 options shown');
	});
});
