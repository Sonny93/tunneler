import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildOpener } from '#/testing/factories';
import {
	describeOpenAction,
	describeTunnelOpenCommand,
	getBrowserChoice,
	listAlternativeChoices,
	listOpenChoices,
	toDefaultChoice,
} from '#/openers/open_choices';

const vnc = buildOpener();
const rdp = buildOpener({ id: 'freerdp', name: 'FreeRDP' });
const browser = () => getBrowserChoice(i18n);

describe('open choices', () => {
	it('should word the browser itself whatever the daemon says', () => {
		expect(
			toDefaultChoice({ id: 'browser', name: 'Navigateur' }, i18n)
		).toEqual({ id: 'browser', name: 'Browser' });
	});

	it('should keep the name of an application default', () => {
		expect(toDefaultChoice({ id: vnc.id, name: vnc.name }, i18n)).toEqual({
			id: vnc.id,
			name: vnc.name,
		});
	});

	it('should list the browser before the applications', () => {
		expect(listOpenChoices([vnc, rdp], i18n).map(({ id }) => id)).toEqual([
			'browser',
			'remmina-vnc',
			'freerdp',
		]);
	});

	it('should leave the default out of the alternatives', () => {
		expect(
			listAlternativeChoices(
				{ id: vnc.id, name: vnc.name },
				[vnc, rdp],
				i18n
			).map(({ id }) => id)
		).toEqual(['browser', 'freerdp']);
	});

	it('should offer the applications as alternatives to the browser default', () => {
		expect(
			listAlternativeChoices(
				{ id: 'browser', name: 'Browser' },
				[vnc],
				i18n
			).map(({ id }) => id)
		).toEqual(['remmina-vnc']);
	});

	it('should describe opening in the browser', () => {
		expect(describeOpenAction(browser(), i18n)).toBe('Open in the browser');
	});

	it('should describe opening with an application', () => {
		expect(describeOpenAction({ id: vnc.id, name: vnc.name }, i18n)).toBe(
			'Open with Remmina (VNC)'
		);
	});

	it('should describe the palette command of an application', () => {
		expect(
			describeTunnelOpenCommand(
				'5999 → nova:5999',
				{ id: vnc.id, name: vnc.name },
				i18n
			)
		).toBe('Open 5999 → nova:5999 with Remmina (VNC)');
	});

	it('should describe the palette command of the browser', () => {
		expect(describeTunnelOpenCommand('3333 → nova:3333', browser(), i18n)).toBe(
			'Open 3333 → nova:3333 in the browser'
		);
	});
});
