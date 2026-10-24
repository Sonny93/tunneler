import type { X11ProbeDto, X11ServerStartDto } from '#/daemon/protocol';

export type X11ServerAction =
	| { kind: 'proceed' }
	| { kind: 'start'; server: string }
	| { kind: 'ask'; server: string }
	| { kind: 'warnNotRunning' }
	| { kind: 'warnMissing' };

/** What to do before using X11, given what the probe found and the user's setting. */
export function decideX11ServerAction(
	probe: Readonly<X11ProbeDto>,
	setting: X11ServerStartDto
): X11ServerAction {
	if (probe.isReachable) return { kind: 'proceed' };
	if (probe.server === null) return { kind: 'warnMissing' };
	if (setting === 'always') return { kind: 'start', server: probe.server };
	if (setting === 'ask') return { kind: 'ask', server: probe.server };
	return { kind: 'warnNotRunning' };
}
