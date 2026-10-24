import { describe, expect, it } from 'vitest';

import {
	decideX11ServerAction,
	type X11ServerAction,
} from '#/x11/decide_x11_server_action';

const reachable = { display: ':0', isReachable: true, server: null };
const unreachableWithServer = {
	display: ':0',
	isReachable: false,
	server: 'VcXsrv',
};
const unreachableWithoutServer = {
	display: ':0',
	isReachable: false,
	server: null,
};

describe('decideX11ServerAction', () => {
	it.each<['ask' | 'always' | 'never']>([['ask'], ['always'], ['never']])(
		'should proceed when the display is reachable and the setting is %s',
		(setting) => {
			expect(decideX11ServerAction(reachable, setting)).toEqual({
				kind: 'proceed',
			});
		}
	);

	it.each<['ask' | 'always' | 'never', X11ServerAction]>([
		['always', { kind: 'start', server: 'VcXsrv' }],
		['ask', { kind: 'ask', server: 'VcXsrv' }],
		['never', { kind: 'warnNotRunning' }],
	])(
		'should decide for an installed server when the setting is %s',
		(setting, expected) => {
			expect(decideX11ServerAction(unreachableWithServer, setting)).toEqual(
				expected
			);
		}
	);

	it.each<['ask' | 'always' | 'never']>([['ask'], ['always'], ['never']])(
		'should warn that no server is installed when the setting is %s',
		(setting) => {
			expect(decideX11ServerAction(unreachableWithoutServer, setting)).toEqual({
				kind: 'warnMissing',
			});
		}
	);
});
