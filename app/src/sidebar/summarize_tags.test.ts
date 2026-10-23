import { describe, expect, it } from 'vitest';

import { summarizeTags } from '#/sidebar/summarize_tags';
import { buildTunnel } from '#/testing/factories';

describe('summarizeTags', () => {
	it('should count running and total tunnels per tag, sorted ignoring case', () => {
		const summaries = summarizeTags([
			buildTunnel({ id: '1', tags: ['dev', 'Db'], state: { kind: 'active' } }),
			buildTunnel({ id: '2', tags: ['dev'] }),
			buildTunnel({ id: '3', tags: [] }),
		]);

		expect(summaries).toEqual([
			{ tag: 'Db', running: 1, started: 1, total: 1 },
			{ tag: 'dev', running: 1, started: 1, total: 2 },
		]);
	});

	it('should count a tunnel once when it repeats a tag', () => {
		expect(summarizeTags([buildTunnel({ tags: ['dev', 'dev'] })])).toEqual([
			{ tag: 'dev', running: 0, started: 0, total: 1 },
		]);
	});

	it('should count a standby tunnel as started but not as running', () => {
		expect(
			summarizeTags([
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
			])
		).toEqual([{ tag: 'dev', running: 0, started: 1, total: 1 }]);
	});

	it('should return nothing when no tunnel has a tag', () => {
		expect(summarizeTags([buildTunnel()])).toEqual([]);
	});
});
