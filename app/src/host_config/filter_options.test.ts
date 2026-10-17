import { describe, expect, it } from 'vitest';

import { filterOptions } from '#/host_config/filter_options';

const OPTIONS = [
	{ key: 'hostname', value: 'nova.example.com' },
	{ key: 'user', value: 'Jules' },
	{ key: 'port', value: '22' },
];

describe('filterOptions', () => {
	it('should keep every option for a blank query', () => {
		expect(filterOptions(OPTIONS, '  ')).toEqual(OPTIONS);
	});

	it('should match the key ignoring case', () => {
		expect(filterOptions(OPTIONS, 'HOSTNAME')).toEqual([OPTIONS[0]]);
	});

	it('should match the value ignoring case', () => {
		expect(filterOptions(OPTIONS, 'jules')).toEqual([OPTIONS[1]]);
	});

	it('should return nothing when no option matches', () => {
		expect(filterOptions(OPTIONS, 'nope')).toEqual([]);
	});
});
