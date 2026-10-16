import {
	useRef,
	useState,
	type RefObject,
	type KeyboardEvent,
	type SyntheticEvent,
} from 'react';

import { resolveRovingIndex } from '#/tunnels/list/resolve_roving_index';

export const ROVING_CONTROL_ATTRIBUTE = 'data-roving-control';
export const ROVING_ROW_ATTRIBUTE = 'data-row-id';

type UseRovingRowsReturn = {
	activeRowId: string | null;
	containerProps: {
		ref: RefObject<HTMLDivElement | null>;
		onFocusCapture: (event: SyntheticEvent<HTMLElement>) => void;
		onClickCapture: (event: SyntheticEvent<HTMLElement>) => void;
		onKeyDownCapture: (event: KeyboardEvent<HTMLElement>) => void;
	};
};

function findRovingControl(target: EventTarget): HTMLElement | null {
	if (!(target instanceof HTMLElement)) return null;
	return target.closest<HTMLElement>(`[${ROVING_CONTROL_ATTRIBUTE}]`);
}

function readRowId(control: HTMLElement): string | null {
	const row = control.closest<HTMLElement>(`[${ROVING_ROW_ATTRIBUTE}]`);
	return row?.getAttribute(ROVING_ROW_ATTRIBUTE) ?? null;
}

function hasModifier(event: Readonly<KeyboardEvent<HTMLElement>>): boolean {
	return event.ctrlKey || event.altKey || event.metaKey || event.shiftKey;
}

/**
 * Makes the rows of a list a single tab stop per control column. Controls are
 * marked `data-roving-control="<column>"` inside a `data-row-id="<id>"`
 * element. The arrow handlers run in the capture phase and stop propagation
 * so they win over the controls' own handlers (the `Menu` trigger opens on
 * ArrowDown otherwise); keys pressed inside an open menu never match a
 * roving control and are left alone.
 */
export function useRovingRows(rowIds: readonly string[]): UseRovingRowsReturn {
	const containerRef = useRef<HTMLDivElement>(null);
	const [requestedRowId, setRequestedRowId] = useState<string | null>(null);
	const isRequestedRowPresent =
		requestedRowId !== null && rowIds.includes(requestedRowId);
	const activeRowId = isRequestedRowPresent
		? requestedRowId
		: (rowIds[0] ?? null);

	const activateRowOf = (event: SyntheticEvent<HTMLElement>) => {
		const control = findRovingControl(event.target);
		if (control) setRequestedRowId(readRowId(control));
	};

	const handleKeyDownCapture = (event: KeyboardEvent<HTMLElement>) => {
		const control = findRovingControl(event.target);
		const container = containerRef.current;
		if (!control || !container || hasModifier(event)) return;
		const column = control.getAttribute(ROVING_CONTROL_ATTRIBUTE);
		const controls = [
			...container.querySelectorAll<HTMLElement>(
				`[${ROVING_CONTROL_ATTRIBUTE}="${column}"]`
			),
		];
		const targetIndex = resolveRovingIndex(
			event.key,
			controls.indexOf(control),
			controls.length
		);
		if (targetIndex === null) return;
		event.preventDefault();
		event.stopPropagation();
		controls[targetIndex]?.focus();
	};

	return {
		activeRowId,
		containerProps: {
			ref: containerRef,
			onFocusCapture: activateRowOf,
			onClickCapture: activateRowOf,
			onKeyDownCapture: handleKeyDownCapture,
		},
	};
}
