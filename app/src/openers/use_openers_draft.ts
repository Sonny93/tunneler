import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';
import { useState } from 'react';

import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';
import { areOpenerListsEqual } from '#/openers/are_opener_lists_equal';
import { stripOpenerSource } from '#/openers/strip_opener_source';
import { useDaemonStore } from '#/state/daemon_store';

/** The form is hidden, adds an application (empty or prefilled with a `draft`), or edits the one with this identifier. */
export type OpenerEditing =
	| { kind: 'none' }
	| { kind: 'create'; draft?: OpenerSpecDto }
	| { kind: 'edit'; id: string };

export type UseOpenersDraftReturn = {
	/** The detected applications the draft does not override. */
	detectedOpeners: readonly AvailableOpenerDto[];
	/** The user's applications as they would be saved: nothing reaches the daemon from here. */
	userOpeners: readonly OpenerSpecDto[];
	editing: OpenerEditing;
	isChanged: boolean;
	/** Whether the application form holds input that is not in the draft yet. */
	hasFormInput: boolean;
	reportFormInput: (hasInput: boolean) => void;
	startCreating: () => void;
	/** Opens the creation form prefilled with a detected application, which the saved one replaces. */
	startCustomizing: (opener: Readonly<AvailableOpenerDto>) => void;
	startEditing: (id: string) => void;
	stopEditing: () => void;
	save: (spec: OpenerSpecDto) => void;
	/** Resolves to whether the application left the draft (a declined confirmation keeps it). */
	remove: (opener: Readonly<OpenerSpecDto>) => Promise<boolean>;
};

function listUserOpeners(
	openers: readonly AvailableOpenerDto[]
): OpenerSpecDto[] {
	return openers
		.filter(({ source }) => source === 'user')
		.map(stripOpenerSource);
}

/** The user's applications edited in memory until the settings are saved. */
export function useOpenersDraft(): UseOpenersDraftReturn {
	const { t } = useLingui();
	const openers = useDaemonStore((state) => state.openers);
	const tunnels = useDaemonStore((state) => state.tunnels);
	const [initialOpeners] = useState(() => listUserOpeners(openers));
	const [userOpeners, setUserOpeners] = useState(initialOpeners);
	const [editing, setEditing] = useState<OpenerEditing>({ kind: 'none' });
	const [hasFormInput, setHasFormInput] = useState(false);

	const detectedOpeners = openers.filter(
		({ id, source }) =>
			source === 'detected' && !userOpeners.some((opener) => opener.id === id)
	);

	const startEditingWith = (next: OpenerEditing) => {
		setHasFormInput(false);
		setEditing(next);
	};

	const save = (spec: OpenerSpecDto) => {
		const isEdition = editing.kind === 'edit';
		setUserOpeners((current) =>
			isEdition
				? current.map((opener) => (opener.id === spec.id ? spec : opener))
				: [...current, spec]
		);
		startEditingWith({ kind: 'none' });
	};

	const confirmRemoval = async (opener: Readonly<OpenerSpecDto>) => {
		const usageCount = tunnels.filter(
			(tunnel) => tunnel.open?.with === opener.id
		).length;
		if (usageCount === 0) return true;
		const { name } = opener;
		return ConfirmModal.call({
			title: t`Delete the application ${name}?`,
			children: t`${plural(usageCount, {
				one: '# tunnel uses this application and will not be able to open until another one is chosen.',
				other:
					'# tunnels use this application and will not be able to open until another one is chosen.',
			})}`,
			confirmLabel: t`Delete`,
			confirmColor: 'danger',
		});
	};

	const remove = async (opener: Readonly<OpenerSpecDto>) => {
		if (!(await confirmRemoval(opener))) return false;
		setUserOpeners((current) => current.filter(({ id }) => id !== opener.id));
		return true;
	};

	return {
		detectedOpeners,
		userOpeners,
		editing,
		isChanged: !areOpenerListsEqual(userOpeners, initialOpeners),
		hasFormInput,
		reportFormInput: setHasFormInput,
		startCreating: () => startEditingWith({ kind: 'create' }),
		startCustomizing: (opener) =>
			startEditingWith({
				kind: 'create',
				draft: stripOpenerSource(opener),
			}),
		startEditing: (id) => startEditingWith({ kind: 'edit', id }),
		stopEditing: () => startEditingWith({ kind: 'none' }),
		save,
		remove,
	};
}
