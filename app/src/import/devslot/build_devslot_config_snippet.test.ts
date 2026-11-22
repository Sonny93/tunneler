import { describe, expect, it } from 'vitest';

import { buildDevslotConfigSnippet } from '#/import/devslot/build_devslot_config_snippet';

describe('buildDevslotConfigSnippet', () => {
	it('should write the table of the host and the path to fill in', () => {
		expect(buildDevslotConfigSnippet('vega')).toBe(
			'[hosts.vega]\ndevslot_path = "/path/to/ds"'
		);
	});

	it('should quote an alias that is not a bare TOML key', () => {
		expect(buildDevslotConfigSnippet('my host."x"')).toBe(
			'[hosts."my host.\\"x\\""]\ndevslot_path = "/path/to/ds"'
		);
	});
});
