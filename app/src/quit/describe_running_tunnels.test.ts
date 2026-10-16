import { describe, expect, it } from 'vitest';

import { describeTunnelsToStop } from '#/quit/describe_running_tunnels';

describe('describeTunnelsToStop', () => {
	it('should use the singular for one tunnel', () => {
		expect(describeTunnelsToStop(1)).toBe('1 tunnel actif sera arrêté.');
	});

	it('should use the plural for several tunnels', () => {
		expect(describeTunnelsToStop(3)).toBe('3 tunnels actifs seront arrêtés.');
	});
});
