import type { I18n } from '@lingui/core';
import { msg, plural } from '@lingui/core/macro';

import type {
	DevslotFailedPortDto,
	DevslotSkippedPortDto,
	DevslotUpResultDto,
} from '#/daemon/protocol';
import { describeRemovedStale } from '#/import/devslot/devslot_texts';

/** What `devslot.up` did: `summary` is what worked, `problems` the ports that did not come up. */
export type DevslotUpReport = {
	summary: string;
	problems: readonly string[];
};

function describeTunnels(
	result: Readonly<DevslotUpResultDto>,
	i18n: I18n
): string {
	const startedCount = result.started.length;
	const existingCount = result.existing.length;
	const parts: string[] = [];
	if (startedCount > 0) {
		parts.push(
			i18n._(
				msg`${plural(startedCount, {
					one: '# tunnel started',
					other: '# tunnels started',
				})}`
			)
		);
	}
	if (existingCount > 0) {
		parts.push(
			i18n._(
				msg`${plural(existingCount, {
					one: '# tunnel already running',
					other: '# tunnels already running',
				})}`
			)
		);
	}
	if (parts.length === 0) return i18n._(msg`No tunnel was started.`);
	return `${parts.join(', ')}.`;
}

function describeOpening(
	result: Readonly<DevslotUpResultDto>,
	i18n: I18n
): string {
	const { opened, openError } = result;
	if (opened !== undefined) {
		return i18n._(msg`Opened ${opened} in the browser.`);
	}
	if (openError !== undefined) {
		return i18n._(msg`The browser could not be opened: ${openError}`);
	}
	return i18n._(msg`No web address was found to open.`);
}

function describeNeedsReview(count: number, i18n: I18n): string {
	return i18n._(
		msg`${plural(count, {
			one: '# stale tunnel needs your review in the import panel (From devslot).',
			other:
				'# stale tunnels need your review in the import panel (From devslot).',
		})}`
	);
}

function describeFailedPort(
	failure: Readonly<DevslotFailedPortDto>,
	i18n: I18n
): string {
	const { remotePort, message } = failure;
	return i18n._(msg`Port ${remotePort} could not be started: ${message}`);
}

function describeSkippedPort(
	skipped: Readonly<DevslotSkippedPortDto>,
	i18n: I18n
): string {
	const { remotePort } = skipped;
	switch (skipped.reason) {
		case 'localPortBusy':
			return i18n._(
				msg`Port ${remotePort} was skipped: no free port was found on this computer.`
			);
		case 'needsReview':
			return i18n._(
				msg`Port ${remotePort} was skipped: a stale tunnel for it needs your review in the import panel (From devslot).`
			);
		case 'existingTunnel':
			return i18n._(
				msg`Port ${remotePort} was skipped: a tunnel you configured already forwards it. Start that tunnel yourself.`
			);
		case 'privilegedPort':
			return '';
	}
}

function describeSideEffects(
	result: Readonly<DevslotUpResultDto>,
	i18n: I18n
): string[] {
	const sentences: string[] = [];
	const privilegedPorts = result.skipped
		.filter(({ reason }) => reason === 'privilegedPort')
		.map(({ remotePort }) => remotePort);
	if (privilegedPorts.length > 0) {
		const ports = privilegedPorts.join(', ');
		sentences.push(
			i18n._(
				msg`Ports below 1024 are left to the import panel (From devslot): ${ports}.`
			)
		);
	}
	if (result.removed.length > 0) {
		const routes = result.removed.map(
			({ listenPort, remotePort }) => `${listenPort} → ${remotePort}`
		);
		sentences.push(describeRemovedStale(routes, i18n));
	}
	if (result.needsReview > 0) {
		sentences.push(describeNeedsReview(result.needsReview, i18n));
	}
	if (result.appPortShared) {
		sentences.push(
			i18n._(
				msg`Another instance lists the same application port: the browser may show that one.`
			)
		);
	}
	return sentences;
}

/** Describes the outcome of `devslot.up`; `problems` is empty when every port is up. */
export function describeDevslotUp(
	result: Readonly<DevslotUpResultDto>,
	i18n: I18n
): DevslotUpReport {
	const { instance, host } = result;
	const sentences = [
		i18n._(msg`${instance} on ${host}:`),
		describeTunnels(result, i18n),
		describeOpening(result, i18n),
		...describeSideEffects(result, i18n),
	];
	return {
		summary: sentences.join(' '),
		problems: [
			...result.failed.map((failure) => describeFailedPort(failure, i18n)),
			...result.skipped
				.filter(({ reason }) => reason !== 'privilegedPort')
				.map((skipped) => describeSkippedPort(skipped, i18n)),
		],
	};
}
