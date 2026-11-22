import { useLingui } from '@lingui/react/macro';
import { useState } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import type { DevslotUpResultDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { describeDevslotUp } from '#/devslot_link/describe_devslot_up';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { pushToast } from '#/shared/toasts/push_toast';
import { useUiStore } from '#/state/ui_store';

export type OpenPhase =
	| { kind: 'confirm' }
	| { kind: 'running' }
	| {
			kind: 'failed';
			/** Names the problem before the list, so it is never conveyed by position alone. */
			leadIn: string;
			/** What worked, when part of the instance came up. */
			summary: string | null;
			problems: readonly string[];
	  };

type OpenTarget = {
	requestId: number;
	host: string;
	instance: string;
};

type UseOpenDevslotInstanceReturn = {
	phase: OpenPhase;
	open: () => Promise<void>;
};

/**
 * Runs `devslot.up` for the dialog of request `requestId`. Closing the dialog
 * while the call runs does not cancel it: the outcome then comes as a toast.
 * Problems stay in the dialog while it is open.
 */
export function useOpenDevslotInstance({
	requestId,
	host,
	instance,
}: Readonly<OpenTarget>): UseOpenDevslotInstanceReturn {
	const { t, i18n } = useLingui();
	const gateway = useDaemonGateway();
	const [phase, setPhase] = useState<OpenPhase>({ kind: 'confirm' });

	const isDialogOpen = () =>
		useUiStore.getState().devslotLinkRequest?.id === requestId;

	const reportResult = (result: Readonly<DevslotUpResultDto>) => {
		const { summary, problems } = describeDevslotUp(result, i18n);
		if (problems.length === 0) {
			pushToast({ tone: 'success', message: summary });
			useUiStore.getState().closeDevslotLink(requestId);
			return;
		}
		const leadIn = t`Some ports are not up:`;
		if (isDialogOpen()) {
			setPhase({ kind: 'failed', leadIn, summary, problems });
			return;
		}
		pushToast({
			tone: 'warning',
			message: [summary, leadIn, ...problems].join(' '),
		});
	};

	const reportFailure = (description: string) => {
		if (isDialogOpen()) {
			setPhase({
				kind: 'failed',
				leadIn: t`Couldn't open ${instance} on ${host}.`,
				summary: null,
				problems: [description],
			});
			return;
		}
		pushToast({
			tone: 'error',
			message: t`Couldn't open ${instance} on ${host}: ${description}`,
		});
	};

	const open = async () => {
		setPhase({ kind: 'running' });
		useAnnouncerStore
			.getState()
			.announce(t`Opening ${instance} on ${host}…`, 'polite');
		try {
			reportResult(await gateway.devslotUp(host, instance, true));
		} catch (rejection) {
			reportFailure(describeDaemonError(toDaemonCommandError(rejection), i18n));
		}
	};

	return { phase, open };
}
