import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';
import { useState } from 'react';

import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';

/** The form is hidden, adds an application (empty or prefilled with a `draft`), or edits the one with this identifier. */
export type OpenerEditing =
	| { kind: 'none' }
	| { kind: 'create'; draft?: OpenerSpecDto }
	| { kind: 'edit'; id: string };

type UseOpenersEditorReturn = {
	detectedOpeners: readonly AvailableOpenerDto[];
	userOpeners: readonly AvailableOpenerDto[];
	editing: OpenerEditing;
	startCreating: () => void;
	/** Opens the creation form prefilled with a detected application, which the saved one replaces. */
	startCustomizing: (opener: Readonly<AvailableOpenerDto>) => void;
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

export function useOpenersEditor(): UseOpenersEditorReturn {
	const { t } = useLingui();
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
		const { name } = spec;
		useToastStore.getState().pushToast({
			tone: 'success',
			message: isEdition
				? t`Application ${name} saved`
				: t`Application ${name} added`,
		});
		setEditing({ kind: 'none' });
	};

	const remove = async (opener: Readonly<OpenerSpecDto>) => {
		const usageCount = tunnels.filter(
			(tunnel) => tunnel.open?.with === opener.id
		).length;
		const { name } = opener;
		if (usageCount > 0) {
			const isConfirmed = await ConfirmModal.call({
				title: t`Delete the application ${name}?`,
				children: t`${plural(usageCount, {
					one: '# tunnel uses this application and will not be able to open until another one is chosen.',
					other:
						'# tunnels use this application and will not be able to open until another one is chosen.',
				})}`,
				confirmLabel: t`Delete`,
				confirmColor: 'danger',
			});
			if (!isConfirmed) return;
		}
		const remaining = userOpeners
			.filter(({ id }) => id !== opener.id)
			.map(toSpec);
		const isDeleted = await runDaemonAction(
			(daemon) => daemon.updateOpeners(remaining),
			t`Couldn't delete the application`
		);
		if (!isDeleted) return;
		useToastStore.getState().pushToast({
			tone: 'success',
			message: t`Application ${name} deleted`,
		});
	};

	return {
		detectedOpeners,
		userOpeners,
		editing,
		startCreating: () => setEditing({ kind: 'create' }),
		startCustomizing: (opener) =>
			setEditing({ kind: 'create', draft: toSpec(opener) }),
		startEditing: (id) => setEditing({ kind: 'edit', id }),
		stopEditing: () => setEditing({ kind: 'none' }),
		save,
		remove,
	};
}
