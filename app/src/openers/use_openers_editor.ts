import { useState } from 'react';
import { ConfirmModal } from '@minimalstuff/ui';

import { useDaemonStore } from '#/state/daemon_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';

/** The form is hidden, adds an application, or edits the one with this identifier. */
export type OpenerEditing =
	| { kind: 'none' }
	| { kind: 'create' }
	| { kind: 'edit'; id: string };

type UseOpenersEditorReturn = {
	detectedOpeners: readonly AvailableOpenerDto[];
	userOpeners: readonly AvailableOpenerDto[];
	editing: OpenerEditing;
	startCreating: () => void;
	startEditing: (id: string) => void;
	stopEditing: () => void;
	/** Rejects when the daemon refuses, so the form can show why. */
	save: (spec: OpenerSpecDto) => Promise<void>;
	remove: (opener: Readonly<OpenerSpecDto>) => Promise<void>;
};

function toSpec(opener: Readonly<AvailableOpenerDto>): OpenerSpecDto {
	const { id, name, command, args, ports } = opener;
	return { id, name, command, args, ports };
}

function describeUsage(count: number): string {
	return count === 1
		? '1 tunnel utilise cette application et ne pourra plus s’ouvrir tant qu’une autre n’est pas choisie.'
		: `${count} tunnels utilisent cette application et ne pourront plus s’ouvrir tant qu’une autre n’est pas choisie.`;
}

export function useOpenersEditor(): UseOpenersEditorReturn {
	const gateway = useDaemonGateway();
	const runDaemonAction = useDaemonAction();
	const openers = useDaemonStore((state) => state.openers);
	const tunnels = useDaemonStore((state) => state.tunnels);
	const [editing, setEditing] = useState<OpenerEditing>({ kind: 'none' });
	const userOpeners = openers.filter(({ source }) => source === 'user');
	const detectedOpeners = openers.filter(({ source }) => source === 'detected');

	const save = async (spec: OpenerSpecDto) => {
		const others = userOpeners.map(toSpec);
		const isEdition = editing.kind === 'edit';
		const next = isEdition
			? others.map((opener) => (opener.id === spec.id ? spec : opener))
			: [...others, spec];
		await gateway.updateOpeners(next);
		useToastStore.getState().pushToast({
			tone: 'success',
			message: `Application ${spec.name} ${isEdition ? 'enregistrée' : 'ajoutée'}`,
		});
		setEditing({ kind: 'none' });
	};

	const remove = async (opener: Readonly<OpenerSpecDto>) => {
		const usageCount = tunnels.filter(
			(tunnel) => tunnel.open?.with === opener.id
		).length;
		if (usageCount > 0) {
			const isConfirmed = await ConfirmModal.call({
				title: `Supprimer l’application ${opener.name} ?`,
				children: describeUsage(usageCount),
				confirmLabel: 'Supprimer',
				confirmColor: 'danger',
			});
			if (!isConfirmed) return;
		}
		const remaining = userOpeners
			.filter(({ id }) => id !== opener.id)
			.map(toSpec);
		const isDeleted = await runDaemonAction(
			(daemon) => daemon.updateOpeners(remaining),
			'Suppression impossible'
		);
		if (!isDeleted) return;
		useToastStore.getState().pushToast({
			tone: 'success',
			message: `Application ${opener.name} supprimée`,
		});
	};

	return {
		detectedOpeners,
		userOpeners,
		editing,
		startCreating: () => setEditing({ kind: 'create' }),
		startEditing: (id) => setEditing({ kind: 'edit', id }),
		stopEditing: () => setEditing({ kind: 'none' }),
		save,
		remove,
	};
}
