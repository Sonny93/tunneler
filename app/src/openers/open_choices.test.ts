import { describe, expect, it } from 'vitest';

import { buildOpener } from '#/testing/factories';
import {
	BROWSER_CHOICE,
	describeOpenAction,
	describeTunnelOpenCommand,
	listAlternativeChoices,
	listOpenChoices,
	toDefaultChoice,
} from '#/openers/open_choices';

const vnc = buildOpener();
const rdp = buildOpener({ id: 'freerdp', name: 'FreeRDP' });

describe('open choices', () => {
	it('should word the browser in French whatever the daemon says', () => {
		expect(toDefaultChoice({ id: 'browser', name: 'Browser' })).toEqual(
			BROWSER_CHOICE
		);
	});

	it('should keep the name of an application default', () => {
		expect(toDefaultChoice({ id: vnc.id, name: vnc.name })).toEqual({
			id: vnc.id,
			name: vnc.name,
		});
	});

	it('should list the browser before the applications', () => {
		expect(listOpenChoices([vnc, rdp]).map(({ id }) => id)).toEqual([
			'browser',
			'remmina-vnc',
			'freerdp',
		]);
	});

	it('should leave the default out of the alternatives', () => {
		expect(
			listAlternativeChoices({ id: vnc.id, name: vnc.name }, [vnc, rdp]).map(
				({ id }) => id
			)
		).toEqual(['browser', 'freerdp']);
	});

	it('should offer the applications as alternatives to the browser default', () => {
		expect(
			listAlternativeChoices({ id: 'browser', name: 'Browser' }, [vnc]).map(
				({ id }) => id
			)
		).toEqual(['remmina-vnc']);
	});

	it('should describe opening in the browser', () => {
		expect(describeOpenAction(BROWSER_CHOICE)).toBe(
			'Ouvrir dans le navigateur'
		);
	});

	it('should describe opening with an application', () => {
		expect(describeOpenAction({ id: vnc.id, name: vnc.name })).toBe(
			'Ouvrir avec Remmina (VNC)'
		);
	});

	it('should describe the palette command of an application', () => {
		expect(
			describeTunnelOpenCommand('5999 → nova:5999', {
				id: vnc.id,
				name: vnc.name,
			})
		).toBe('Ouvrir 5999 → nova:5999 avec Remmina (VNC)');
	});

	it('should describe the palette command of the browser', () => {
		expect(describeTunnelOpenCommand('3333 → nova:3333', BROWSER_CHOICE)).toBe(
			'Ouvrir 3333 → nova:3333 dans le navigateur'
		);
	});
});
