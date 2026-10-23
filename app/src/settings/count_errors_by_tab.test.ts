import { msg } from '@lingui/core/macro';
import { describe, expect, it } from 'vitest';

import { countErrorsByTab } from '#/settings/count_errors_by_tab';

const ERROR = msg`Invalid`;

describe('countErrorsByTab', () => {
	it('should be empty without errors', () => {
		expect(countErrorsByTab({})).toEqual({});
	});

	it('should count the invalid fields of each tab', () => {
		expect(
			countErrorsByTab({
				defaultBindAddress: ERROR,
				keepaliveMax: ERROR,
				standbyIdleSeconds: ERROR,
			})
		).toEqual({ connection: 2, standby: 1 });
	});
});
