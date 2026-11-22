import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

import type { DevslotWarningDto } from '#/daemon/protocol';

export function describeDevslotFound(
	instanceCount: number,
	i18n: I18n
): string {
	return i18n._(
		msg`${plural(instanceCount, {
			0: 'No devslot instance found',
			one: '# devslot instance found',
			other: '# devslot instances found',
		})}`
	);
}

export function describeDiscovering(host: string, i18n: I18n): string {
	return i18n._(msg`Discovering on ${host}…`);
}

export function describeStarting(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: 'Starting # tunnel…',
			other: 'Starting # tunnels…',
		})}`
	);
}

export function describeStartOutcome(
	startedCount: number,
	failedCount: number,
	i18n: I18n
): string {
	return i18n._(msg`${startedCount} started, ${failedCount} failed`);
}

export function describeRemovedStale(
	routes: readonly string[],
	i18n: I18n
): string {
	const count = routes.length;
	const routeList = routes.join(', ');
	return i18n._(
		msg`${plural(count, {
			one: '# stale tunnel removed: ',
			other: '# stale tunnels removed: ',
		})}${routeList}`
	);
}

export function describeSkippedStale(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: '# tunnel was left unchanged because it changed since the discovery.',
			other:
				'# tunnels were left unchanged because they changed since the discovery.',
		})}`
	);
}

export function describeStartAction(
	count: number,
	isPersistent: boolean,
	i18n: I18n
): string {
	if (isPersistent) {
		return i18n._(
			msg`${plural(count, {
				one: 'Create and start # tunnel',
				other: 'Create and start # tunnels',
			})}`
		);
	}
	return i18n._(
		msg`${plural(count, {
			one: 'Start # tunnel',
			other: 'Start # tunnels',
		})}`
	);
}

export function describeStartedCount(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: '# tunnel started',
			other: '# tunnels started',
		})}`
	);
}

export function describeDevslotWarning(
	warning: Readonly<DevslotWarningDto>,
	i18n: I18n
): string {
	const { instance } = warning;
	if (warning.kind === 'projectWithoutTag') {
		return i18n._(
			msg`The project of ${instance} has no usable tag: its tunnels only get the devslot tag.`
		);
	}
	const variable = warning.var ?? '?';
	return i18n._(
		msg`Port ${variable} of ${instance} was skipped: it is not a valid port number.`
	);
}
