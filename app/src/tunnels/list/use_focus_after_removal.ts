import { useCallback, useEffect, useRef, type RefObject } from 'react';

import {
	ROVING_CONTROL_ATTRIBUTE,
	ROVING_ROW_ATTRIBUTE,
} from '#/tunnels/list/use_roving_rows';

type PendingRemoval = { removedId: string; successorId: string | null };

function findSuccessorId(
	rowIds: readonly string[],
	removedId: string
): string | null {
	const index = rowIds.indexOf(removedId);
	return rowIds[index + 1] ?? rowIds[index - 1] ?? null;
}

export function focusRowSwitch(container: HTMLElement, rowId: string): void {
	const rows = container.querySelectorAll<HTMLElement>(
		`[${ROVING_ROW_ATTRIBUTE}]`
	);
	const row = [...rows].find(
		(candidate) => candidate.getAttribute(ROVING_ROW_ATTRIBUTE) === rowId
	);
	row
		?.querySelector<HTMLElement>(`[${ROVING_CONTROL_ATTRIBUTE}="switch"]`)
		?.focus();
}

/**
 * Keeps focus inside the page when a row is deleted (WCAG 2.4.3): once the row
 * is gone, focuses the switch of the row that took its place (next, else
 * previous), or calls `onEmptied` when no row is left. The roving active row
 * follows because focusing a control activates its row.
 */
export function useFocusAfterRemoval(
	rowIds: readonly string[],
	containerRef: RefObject<HTMLElement | null>,
	onEmptied: () => void
): (removedId: string) => void {
	const currentRowIdsRef = useRef<readonly string[]>([]);
	const previousRowIdsRef = useRef<readonly string[]>([]);
	const pendingRemovalRef = useRef<PendingRemoval | null>(null);
	const onEmptiedRef = useRef(onEmptied);

	const applyPendingRemoval = useCallback(() => {
		const pending = pendingRemovalRef.current;
		if (!pending || currentRowIdsRef.current.includes(pending.removedId)) {
			return;
		}
		pendingRemovalRef.current = null;
		if (pending.successorId === null) {
			onEmptiedRef.current();
			return;
		}
		if (containerRef.current) {
			focusRowSwitch(containerRef.current, pending.successorId);
		}
	}, [containerRef]);

	useEffect(() => {
		onEmptiedRef.current = onEmptied;
	});

	useEffect(() => {
		previousRowIdsRef.current = currentRowIdsRef.current;
		currentRowIdsRef.current = rowIds;
		applyPendingRemoval();
	}, [rowIds, applyPendingRemoval]);

	return (removedId) => {
		const knownRowIds = currentRowIdsRef.current.includes(removedId)
			? currentRowIdsRef.current
			: previousRowIdsRef.current;
		pendingRemovalRef.current = {
			removedId,
			successorId: findSuccessorId(knownRowIds, removedId),
		};
		applyPendingRemoval();
	};
}
