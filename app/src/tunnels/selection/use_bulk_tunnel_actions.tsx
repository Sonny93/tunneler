import { useContext } from 'react';
import { ConfirmModal, Modal } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { TunnelNameList } from '#/tunnels/selection/tunnel_name_list';
import { BulkTagsContent } from '#/tunnels/selection/bulk_tags_content';
import { runBulkOperation } from '#/tunnels/selection/run_bulk_operation';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';
import { formatTunnelCount } from '#/tunnels/selection/describe_selection_count';
import {
	describeBulkOutcome,
	type BulkParticiple,
} from '#/tunnels/selection/describe_bulk_outcome';
import {
	findTunnelsToRetag,
	toRetaggedDefinition,
	type TagChange,
} from '#/tunnels/selection/tag_change';

export type BulkActionKey = 'start' | 'stop' | 'restart' | 'tags' | 'remove';

export type BulkAction = {
	key: BulkActionKey;
	label: string;
	icon: string;
	isDanger: boolean;
	isDisabled: boolean;
	perform: () => void;
};

const STARTED: BulkParticiple = { singular: 'démarré', plural: 'démarrés' };
const STOPPED: BulkParticiple = { singular: 'arrêté', plural: 'arrêtés' };
const RESTARTED: BulkParticiple = {
	singular: 'redémarré',
	plural: 'redémarrés',
};
const REMOVED: BulkParticiple = { singular: 'supprimé', plural: 'supprimés' };
const UPDATED: BulkParticiple = {
	singular: 'mis à jour',
	plural: 'mis à jour',
};

const MODAL_CLOSE_LABEL = 'Fermer';

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
	const gateway = useDaemonGateway();
	const requestFocusAfterRemoval = useContext(RemovalFocusContext);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);
	const idleTunnels = tunnels.filter(({ state }) => !isTunnelRunning(state));
	const runningTunnels = tunnels.filter(({ state }) => isTunnelRunning(state));

	const runOn = async (
		targets: readonly TunnelDto[],
		perform: (tunnel: TunnelDto) => Promise<unknown>,
		participle: BulkParticiple
	) => {
		const outcome = await runBulkOperation(targets, perform);
		useToastStore
			.getState()
			.pushToast(describeBulkOutcome(outcome, participle));
		return outcome;
	};

	const confirmRemoval = async () => {
		const isConfirmed = await ConfirmModal.call({
			title: `Supprimer ${formatTunnelCount(tunnels.length)} ?`,
			children: <TunnelNameList tunnels={tunnels} />,
			confirmLabel: 'Supprimer',
			cancelLabel: 'Annuler',
			closeLabel: MODAL_CLOSE_LABEL,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const { succeededIds } = await runOn(
			tunnels,
			({ id }) => gateway.deleteTunnel(id),
			REMOVED
		);
		clearTunnelSelection();
		if (succeededIds.length > 0) requestFocusAfterRemoval(succeededIds);
	};

	const askTagChange = async (): Promise<TagChange | null> => {
		const answer: { change: TagChange | null } = { change: null };
		const close = () => Modal.end(call, undefined);
		const call = Modal.call({
			title: `Tags de ${formatTunnelCount(tunnels.length)}`,
			size: 'sm',
			closeLabel: MODAL_CLOSE_LABEL,
			children: (
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
		await call;
		return answer.change;
	};

	const editTags = async () => {
		const change = await askTagChange();
		if (change === null) return;
		const targets = findTunnelsToRetag(tunnels, change);
		if (targets.length === 0) {
			useToastStore
				.getState()
				.pushToast({ tone: 'info', message: 'Aucun tunnel à modifier' });
			return;
		}
		await runOn(
			targets,
			(tunnel) => gateway.updateTunnel(toRetaggedDefinition(tunnel, change)),
			UPDATED
		);
	};

	return [
		{
			key: 'start',
			label: 'Démarrer',
			icon: 'i-mdi-play',
			isDanger: false,
			isDisabled: idleTunnels.length === 0,
			perform: () =>
				void runOn(idleTunnels, ({ id }) => gateway.startTunnel(id), STARTED),
		},
		{
			key: 'stop',
			label: 'Arrêter',
			icon: 'i-mdi-stop',
			isDanger: false,
			isDisabled: runningTunnels.length === 0,
			perform: () =>
				void runOn(runningTunnels, ({ id }) => gateway.stopTunnel(id), STOPPED),
		},
		{
			key: 'restart',
			label: 'Redémarrer',
			icon: 'i-mdi-restart',
			isDanger: false,
			isDisabled: runningTunnels.length === 0,
			perform: () =>
				void runOn(
					runningTunnels,
					({ id }) => gateway.restartTunnel(id),
					RESTARTED
				),
		},
		{
			key: 'tags',
			label: 'Tags…',
			icon: 'i-mdi-tag-multiple-outline',
			isDanger: false,
			isDisabled: false,
			perform: () => void editTags(),
		},
		{
			key: 'remove',
			label: 'Supprimer',
			icon: 'i-mdi-delete-outline',
			isDanger: true,
			isDisabled: false,
			perform: () => void confirmRemoval(),
		},
	];
}
