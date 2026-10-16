import { useEffect, type KeyboardEvent, type RefObject } from 'react';

const FOCUSABLE_SELECTOR =
	'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
const BODY_SELECTOR = '[data-drawer-body]';
const MAIN_SELECTOR = 'main';

function getFocusableElements(container: HTMLElement): HTMLElement[] {
	return [...container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)];
}

function moveFocusIntoPanel(panel: HTMLElement): void {
	const body = panel.querySelector<HTMLElement>(BODY_SELECTOR);
	const [firstBodyElement] = body ? getFocusableElements(body) : [];
	(firstBodyElement ?? panel).focus();
}

function canRestoreFocus(element: Element | null): element is HTMLElement {
	return (
		element instanceof HTMLElement &&
		element.isConnected &&
		element !== document.body
	);
}

type UseDrawerFocusReturn = {
	handleKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
};

/** Moves focus into the panel on mount, restores it on unmount and traps Tab inside. */
export function useDrawerFocus(
	panelRef: RefObject<HTMLElement | null>
): UseDrawerFocusReturn {
	useEffect(() => {
		const previouslyFocused = document.activeElement;
		if (panelRef.current) moveFocusIntoPanel(panelRef.current);
		return () => {
			if (canRestoreFocus(previouslyFocused)) previouslyFocused.focus();
			else document.querySelector<HTMLElement>(MAIN_SELECTOR)?.focus();
		};
	}, [panelRef]);

	const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
		if (event.key !== 'Tab' || !panelRef.current) return;
		const focusableElements = getFocusableElements(panelRef.current);
		const first = focusableElements.at(0);
		const last = focusableElements.at(-1);
		if (!first || !last) {
			event.preventDefault();
			return;
		}
		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
			return;
		}
		if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	};

	return { handleKeyDown };
}
