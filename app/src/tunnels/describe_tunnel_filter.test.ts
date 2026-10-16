import { describe, expect, it } from 'vitest';

import { describeTunnelFilter } from '#/tunnels/describe_tunnel_filter';

describe('describeTunnelFilter', () => {
	it.each([
		[{ kind: 'all' }, 'Tous les tunnels'],
		[{ kind: 'tag', tag: 'dev' }, 'Tag dev'],
		[{ kind: 'host', host: 'nova' }, 'Host nova'],
	] as const)('should describe %j as %s', (filter, description) => {
		expect(describeTunnelFilter(filter)).toBe(description);
	});
});
