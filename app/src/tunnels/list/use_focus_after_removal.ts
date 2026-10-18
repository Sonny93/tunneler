import { useCallback, useEffect, useRef, type RefObject } from 'react';

import {
	ROVING_CONTROL_ATTRIBUTE,
	ROVING_ROW_ATTRIBUTE,
} from '#/tunnels/list/use_roving_rows';

type PendingRemoval = {
	removedIds: readonly string[];
	successorId: string | null;
};

/**
 * The first remaining row after the first removed one, else the last remaining
 * row before it. A removed row already gone from `rowIds` counts as the first.
 */
function findSuccessorId(
	rowIds: readonly string[],
	removedIds: readonly string[]
): string | null {
	const firstRemovedIndex = Math.max(
		rowIds.findIndex((rowId) => removedIds.includes(rowId)),
		0
	);
	const remainingIds = rowIds.filter((rowId) => !removedIds.includes(rowId));
	const nextId = rowIds
		.slice(firstRemovedIndex)
		.find((rowId) => remainingIds.includes(rowId));
	const previousId = rowIds
		.slice(0, firstRemovedIndex)
		.filter((rowId) => remainingIds.includes(rowId))
		.at(-1);
	return nextId ?? previousId ?? null;
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
 * Keeps focus inside the page when rows are deleted (WCAG 2.4.3): once the
 * rows are gone, focuses the switch of the row that took their place (next,
 * else previous), or calls `onEmptied` when no row is left. The roving active row
 * follows because focusing a control activates its row.
 */
export function useFocusAfterRemoval(
	rowIds: readonly string[],
	containerRef: RefObject<HTMLElement | null>,
	onEmptied: () => void
): (removedIds: readonly string[]) => void {
	const currentRowIdsRef = useRef<readonly string[]>([]);
	const previousRowIdsRef = useRef<readonly string[]>([]);
	const pendingRemovalRef = useRef<PendingRemoval | null>(null);
	const onEmptiedRef = useRef(onEmptied);

	const applyPendingRemoval = useCallback(() => {
		const pending = pendingRemovalRef.current;
		if (
			!pending ||
			pending.removedIds.some((removedId) =>
				currentRowIdsRef.current.includes(removedId)
			)
		) {
			return;
		}
		pendingRemovalRef.current = null;
		const currentRowIds = currentRowIdsRef.current;
		const isSuccessorListed =
			pending.successorId !== null &&
			currentRowIds.includes(pending.successorId);
		const successorId = isSuccessorListed
			? pending.successorId
			: (currentRowIds[0] ?? null);
		if (successorId === null) {
			onEmptiedRef.current();
			return;
		}
		if (containerRef.current) {
			focusRowSwitch(containerRef.current, successorId);
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

	return (removedIds) => {
		const isStillListed = removedIds.some((removedId) =>
			currentRowIdsRef.current.includes(removedId)
		);
		const knownRowIds = isStillListed
			? currentRowIdsRef.current
			: previousRowIdsRef.current;
		pendingRemovalRef.current = {
			removedIds,
			successorId: findSuccessorId(knownRowIds, removedIds),
		};
		applyPendingRemoval();
	};
}
