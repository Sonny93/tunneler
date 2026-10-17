import { describe, expect, it } from 'vitest';

import { formatConfigLines } from '#/host_config/format_config_lines';

describe('formatConfigLines', () => {
	it('should print one key value line per option', () => {
		expect(
			formatConfigLines([
				{ key: 'hostname', value: 'nova.example.com' },
				{ key: 'port', value: '22' },
			])
		).toBe('hostname nova.example.com\nport 22');
	});

	it('should print nothing without options', () => {
		expect(formatConfigLines([])).toBe('');
	});
});
