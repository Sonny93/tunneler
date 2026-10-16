import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { resolveTunnelFilter } from '#/tunnels/resolve_tunnel_filter';

const TUNNELS = [buildTunnel({ host: 'nova', tags: ['dev'] })];

describe('resolveTunnelFilter', () => {
	it('should keep a tag filter whose tag exists', () => {
		expect(resolveTunnelFilter(TUNNELS, { kind: 'tag', tag: 'dev' })).toEqual({
			kind: 'tag',
			tag: 'dev',
		});
	});

	it('should fall back to all when the tag no longer exists', () => {
		expect(resolveTunnelFilter(TUNNELS, { kind: 'tag', tag: 'prod' })).toEqual({
			kind: 'all',
		});
	});

	it('should keep a host filter whose host exists', () => {
		expect(
			resolveTunnelFilter(TUNNELS, { kind: 'host', host: 'nova' })
		).toEqual({ kind: 'host', host: 'nova' });
	});

	it('should fall back to all when the host no longer exists', () => {
		expect(
			resolveTunnelFilter(TUNNELS, { kind: 'host', host: 'gone' })
		).toEqual({ kind: 'all' });
	});

	it('should keep the all filter', () => {
		expect(resolveTunnelFilter([], { kind: 'all' })).toEqual({ kind: 'all' });
	});
});
