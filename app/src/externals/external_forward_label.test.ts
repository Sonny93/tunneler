import { describe, expect, it } from 'vitest';

import {
	getExternalForwardAccessibleLabel,
	getExternalForwardLabel,
} from '#/externals/external_forward_label';

describe('getExternalForwardLabel', () => {
	it('should show a local forward with an arrow', () => {
		expect(
			getExternalForwardLabel({
				kind: 'local',
				listenPort: 3333,
				targetHost: 'localhost',
				targetPort: 3333,
			})
		).toBe('-L 3333 → localhost:3333');
	});

	it('should show only the port of a dynamic forward', () => {
		expect(getExternalForwardLabel({ kind: 'dynamic', listenPort: 1080 })).toBe(
			'-D 1080'
		);
	});

	it('should show a remote forward with the -R flag', () => {
		expect(
			getExternalForwardLabel({
				kind: 'remote',
				listenPort: 8080,
				targetHost: 'web',
				targetPort: 80,
			})
		).toBe('-R 8080 → web:80');
	});

	it('should prefix the listen address when there is one', () => {
		expect(
			getExternalForwardLabel({
				kind: 'local',
				listenAddress: '0.0.0.0',
				listenPort: 3333,
				targetHost: 'db',
				targetPort: 5432,
			})
		).toBe('-L 0.0.0.0:3333 → db:5432');
	});

	it('should default the target to localhost and the listen port', () => {
		expect(getExternalForwardLabel({ kind: 'local', listenPort: 3333 })).toBe(
			'-L 3333 → localhost:3333'
		);
	});
});

describe('getExternalForwardAccessibleLabel', () => {
	it('should spell a local forward without an arrow', () => {
		expect(
			getExternalForwardAccessibleLabel({
				kind: 'local',
				listenPort: 3333,
				targetHost: 'localhost',
				targetPort: 3333,
			})
		).toBe('Redirection locale 3333 vers localhost:3333');
	});

	it('should name a dynamic forward', () => {
		expect(
			getExternalForwardAccessibleLabel({ kind: 'dynamic', listenPort: 1080 })
		).toBe('Proxy dynamique 1080');
	});
});
