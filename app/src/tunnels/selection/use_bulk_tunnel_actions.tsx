import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal, Modal } from '@minimalstuff/ui';
import { useContext } from 'react';

import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { confirmStandby } from '#/standby/confirm_standby';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';
import { BulkTagsContent } from '#/tunnels/selection/bulk_tags_content';
import {
	describeBulkOutcome,
	type BulkVerb,
} from '#/tunnels/selection/describe_bulk_outcome';
import { runBulkOperation } from '#/tunnels/selection/run_bulk_operation';
import {
	findTunnelsToRetag,
	toRetaggedDefinition,
	type TagChange,
} from '#/tunnels/selection/tag_change';
import { TunnelLabelList } from '#/tunnels/selection/tunnel_label_list';

export type BulkActionKey =
	| 'start'
	| 'stop'
	| 'restart'
	| 'standby'
	| 'tags'
	| 'remove';

export type BulkAction = {
	key: BulkActionKey;
	label: string;
	icon: string;
	isDanger: boolean;
	isDisabled: boolean;
	perform: () => void;
};

function listKnownTags(tunnels: readonly TunnelDto[]): string[] {
	return [...new Set(tunnels.flatMap(({ tags }) => tags))].sort();
}

/**
 * The bulk actions on `tunnels` (the selection), each applying to the relevant
 * tunnels only: start the ones that are not running, stop and restart the
 * running ones. Every action reports one toast. Must be rendered inside the
 * list so the focus can move on after a deletion.
 */
export function useBulkTunnelActions(
	tunnels: readonly TunnelDto[]
): readonly BulkAction[] {
	const { t, i18n } = useLingui();
	const gateway = useDaemonGateway();
	const requestFocusAfterRemoval = useContext(RemovalFocusContext);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);
	const idleTunnels = tunnels.filter(({ state }) => !isTunnelRunning(state));
	const runningTunnels = tunnels.filter(({ state }) => isTunnelRunning(state));
	const awakeTunnels = listAwakeTunnels(tunnels);
	const stoppableTunnels = tunnels.filter(
		({ state, isTemporary }) => isTemporary || isTunnelRunning(state)
	);
	const hasOnlyTemporaryTunnels = tunnels.every(
		({ isTemporary }) => isTemporary
	);

	const runOn = async (
		targets: readonly TunnelDto[],
		perform: (tunnel: TunnelDto) => Promise<unknown>,
		verb: BulkVerb
	) => {
		const outcome = await runBulkOperation(targets, perform, i18n);
		useToastStore
			.getState()
			.pushToast(describeBulkOutcome(outcome, verb, i18n));
		return outcome;
	};

	const stop = async () => {
		const { succeededIds } = await runOn(
			stoppableTunnels,
			({ id }) => gateway.stopTunnel(id),
			'stop'
		);
		const removedIds = stoppableTunnels
			.filter(({ id, isTemporary }) => isTemporary && succeededIds.includes(id))
			.map(({ id }) => id);
		if (removedIds.length > 0) requestFocusAfterRemoval(removedIds);
	};

	const confirmAndSendToStandby = async () => {
		const awakeCount = awakeTunnels.length;
		const isConfirmed = await confirmStandby(
			t`${plural(awakeCount, {
				one: 'Send # tunnel to standby?',
				other: 'Send # tunnels to standby?',
			})}`,
			awakeTunnels,
			i18n
		);
		if (!isConfirmed) return;
		await runOn(awakeTunnels, ({ id }) => gateway.standbyTunnel(id), 'standby');
	};

	const confirmRemoval = async () => {
		const tunnelCount = tunnels.length;
		const isConfirmed = await ConfirmModal.call({
			title: t`${plural(tunnelCount, {
				one: 'Delete # tunnel?',
				other: 'Delete # tunnels?',
			})}`,
			children: <TunnelLabelList tunnels={tunnels} />,
			confirmLabel: t`Delete`,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const { succeededIds } = await runOn(
			tunnels,
			({ id }) => gateway.deleteTunnel(id),
			'remove'
		);
		clearTunnelSelection();
		if (succeededIds.length > 0) requestFocusAfterRemoval(succeededIds);
	};

	const askTagChange = async (): Promise<TagChange | null> => {
		const answer: { change: TagChange | null } = { change: null };
		const tunnelCount = tunnels.length;
		await Modal.call({
			title: t`${plural(tunnelCount, {
				one: 'Tags for # tunnel',
				other: 'Tags for # tunnels',
			})}`,
			size: 'sm',
			children: (close) => (
				<BulkTagsContent
					presentTags={listKnownTags(tunnels)}
					knownTags={listKnownTags(useDaemonStore.getState().tunnels)}
					onApply={(change) => {
						answer.change = change;
						close();
					}}
					onCancel={close}
				/>
			),
		});
		return answer.change;
	};

	const editTags = async () => {
		const change = await askTagChange();
		if (change === null) return;
		const targets = findTunnelsToRetag(tunnels, change);
		if (targets.length === 0) {
			useToastStore
				.getState()
				.pushToast({ tone: 'info', message: t`No tunnels to update` });
			return;
		}
		await runOn(
			targets,
			(tunnel) => gateway.updateTunnel(toRetaggedDefinition(tunnel, change)),
			'update'
		);
	};

	return [
		{
			key: 'start',
			label: t`Start`,
			icon: 'i-mdi-play',
			isDanger: false,
			isDisabled: idleTunnels.length === 0,
			perform: () =>
				void runOn(idleTunnels, ({ id }) => gateway.startTunnel(id), 'start'),
		},
		{
			key: 'stop',
			label: t`Stop`,
			icon: 'i-mdi-stop',
			isDanger: false,
			isDisabled: stoppableTunnels.length === 0,
			perform: () => void stop(),
		},
		{
			key: 'restart',
			label: t`Restart`,
			icon: 'i-mdi-restart',
			isDanger: false,
			isDisabled: runningTunnels.length === 0,
			perform: () =>
				void runOn(
					runningTunnels,
					({ id }) => gateway.restartTunnel(id),
					'restart'
				),
		},
		{
			key: 'standby',
			label: t`Send to standby`,
			icon: 'i-mdi-sleep',
			isDanger: false,
			isDisabled: awakeTunnels.length === 0,
			perform: () => void confirmAndSendToStandby(),
		},
		{
			key: 'tags',
			label: t`Tags…`,
			icon: 'i-mdi-tag-multiple-outline',
			isDanger: false,
			isDisabled: hasOnlyTemporaryTunnels,
			perform: () => void editTags(),
		},
		{
			key: 'remove',
			label: t`Delete`,
			icon: 'i-mdi-delete-outline',
			isDanger: true,
			isDisabled: false,
			perform: () => void confirmRemoval(),
		},
	];
}
