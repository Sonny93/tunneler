import { describe, expect, it } from 'vitest';

import { buildOpener } from '#/testing/factories';
import { suggestOpener } from '#/tunnel_form/suggest_opener';

const vnc = buildOpener({ ports: [5900, 5901] });
const rdp = buildOpener({ id: 'freerdp', name: 'FreeRDP', ports: [3389] });
const linked = { listenPort: '5901', targetPort: '', isTargetPortLinked: true };

describe('suggestOpener', () => {
	it('should follow the listen port while the target port is linked', () => {
		expect(suggestOpener(linked, [rdp, vnc], 'suggested')).toBe(vnc);
	});

	it('should follow the target port once unlinked', () => {
		expect(
			suggestOpener(
				{ listenPort: '5901', targetPort: '3389', isTargetPortLinked: false },
				[rdp, vnc],
				'suggested'
			)
		).toBe(rdp);
	});

	it('should suggest nothing when no opener matches', () => {
		expect(
			suggestOpener({ ...linked, listenPort: '80' }, [vnc, rdp], 'suggested')
		).toBeNull();
	});

	it('should suggest nothing while the port is invalid', () => {
		expect(
			suggestOpener({ ...linked, listenPort: 'abc' }, [vnc], 'suggested')
		).toBeNull();
	});

	it('should prefer the first opener of the list', () => {
		const mine = buildOpener({ id: 'mine', ports: [5901], source: 'user' });

		expect(suggestOpener(linked, [mine, vnc], 'suggested')).toBe(mine);
	});

	it('should suggest nothing when the general setting opens the browser', () => {
		expect(suggestOpener(linked, [vnc], 'browser')).toBeNull();
	});
});
