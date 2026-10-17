import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import { buildSshCommand } from '#/tunnel_details/build_ssh_command';

describe('buildSshCommand', () => {
	it('should default the target to the loopback of the ssh host and the listen port', () => {
		expect(buildSshCommand(buildTunnel())).toBe(
			'ssh -N -L 3333:127.0.0.1:3333 nova'
		);
	});

	it('should use the target host and port of a local forward', () => {
		expect(
			buildSshCommand(
				buildTunnel({ targetHost: 'db.internal', targetPort: 5432 })
			)
		).toBe('ssh -N -L 3333:db.internal:5432 nova');
	});

	it('should include the listen address when the tunnel sets one', () => {
		expect(
			buildSshCommand(buildTunnel({ listenAddress: '0.0.0.0', targetPort: 80 }))
		).toBe('ssh -N -L 0.0.0.0:3333:127.0.0.1:80 nova');
	});

	it('should bracket an IPv6 listen address', () => {
		expect(buildSshCommand(buildTunnel({ listenAddress: '::1' }))).toBe(
			'ssh -N -L [::1]:3333:127.0.0.1:3333 nova'
		);
	});

	it('should bracket an IPv6 target host', () => {
		expect(buildSshCommand(buildTunnel({ targetHost: '::1' }))).toBe(
			'ssh -N -L 3333:[::1]:3333 nova'
		);
	});

	it('should build a remote forward', () => {
		expect(
			buildSshCommand(
				buildTunnel({ kind: 'remote', listenPort: 8080, targetPort: 3000 })
			)
		).toBe('ssh -N -R 8080:127.0.0.1:3000 nova');
	});

	it('should build a dynamic forward without target', () => {
		expect(
			buildSshCommand(buildTunnel({ kind: 'dynamic', listenPort: 1080 }))
		).toBe('ssh -N -D 1080 nova');
	});

	it('should build a dynamic forward with a listen address', () => {
		expect(
			buildSshCommand(
				buildTunnel({
					kind: 'dynamic',
					listenPort: 1080,
					listenAddress: '127.0.0.1',
				})
			)
		).toBe('ssh -N -D 127.0.0.1:1080 nova');
	});
});
