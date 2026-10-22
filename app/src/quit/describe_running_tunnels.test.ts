import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeTunnelsToStop } from '#/quit/describe_running_tunnels';

describe('describeTunnelsToStop', () => {
	it('should use the singular for one tunnel', () => {
		expect(describeTunnelsToStop(1, i18n)).toBe(
			'1 running tunnel or X11 session will be stopped.'
		);
	});

	it('should use the plural for several tunnels', () => {
		expect(describeTunnelsToStop(3, i18n)).toBe(
			'3 running tunnels or X11 sessions will be stopped.'
		);
	});
});
