import { useEffect, useRef, useState, type RefObject } from 'react';

/** Where the focus goes once the applications section has re-rendered. */
export type OpenerFocusTarget =
	| { kind: 'edit'; id: string }
	| { kind: 'add' }
	| { kind: 'form' };

type UseOpenerFocusReturn = {
	containerRef: RefObject<HTMLElement | null>;
	requestFocus: (target: OpenerFocusTarget) => void;
};

function findFocusTarget(
	container: HTMLElement,
	target: Readonly<OpenerFocusTarget>
): HTMLElement | null {
	if (target.kind === 'add') {
		return container.querySelector<HTMLElement>('[data-opener-add]');
	}
	if (target.kind === 'form') {
		return container.querySelector<HTMLElement>('[data-opener-form] input');
	}
	const editButtons =
		container.querySelectorAll<HTMLElement>('[data-opener-edit]');
	return (
		[...editButtons].find(
			(button) => button.dataset.openerEdit === target.id
		) ?? null
	);
}

/** Moves the focus to a control of the section after the render that created or removed it. */
export function useOpenerFocus(): UseOpenerFocusReturn {
	const containerRef = useRef<HTMLElement>(null);
	const [target, setTarget] = useState<OpenerFocusTarget | null>(null);

	useEffect(() => {
		if (target === null || containerRef.current === null) return;
		findFocusTarget(containerRef.current, target)?.focus();
	}, [target]);

	return { containerRef, requestFocus: setTarget };
}
