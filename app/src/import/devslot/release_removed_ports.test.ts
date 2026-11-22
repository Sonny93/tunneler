import { describe, expect, it } from 'vitest';

import { releaseRemovedPorts } from '#/import/devslot/release_removed_ports';
import { buildDevslotInstance, buildDevslotPort } from '#/testing/factories';

const tunneled = (tunnelId: string) =>
	buildDevslotPort({ status: 'alreadyTunneled', tunnelId });

describe('releaseRemovedPorts', () => {
	it('should make a port new again when its tunnel was removed', () => {
		const instances = [buildDevslotInstance({ ports: [tunneled('gone')] })];

		const [instance] = releaseRemovedPorts(instances, new Set(['gone']));

		expect(instance?.ports[0]).toMatchObject({ status: 'new' });
		expect(instance?.ports[0]).not.toHaveProperty('tunnelId');
	});

	it('should keep a port whose tunnel was not removed', () => {
		const instances = [buildDevslotInstance({ ports: [tunneled('kept')] })];

		const [instance] = releaseRemovedPorts(instances, new Set(['gone']));

		expect(instance?.ports[0]).toMatchObject({
			status: 'alreadyTunneled',
			tunnelId: 'kept',
		});
	});
});
