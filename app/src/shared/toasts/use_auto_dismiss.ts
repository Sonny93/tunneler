import { useEffect } from 'react';

export const TOAST_AUTO_DISMISS_DELAY_MS = 5000;

/** Runs `onDismiss` after the delay; the timer is stopped while `isPaused` (WCAG 2.2.1). */
export function useAutoDismiss(
	isEnabled: boolean,
	isPaused: boolean,
	onDismiss: () => void
): void {
	useEffect(() => {
		if (!isEnabled || isPaused) return;
		const timeoutId = setTimeout(onDismiss, TOAST_AUTO_DISMISS_DELAY_MS);
		return () => clearTimeout(timeoutId);
	}, [isEnabled, isPaused, onDismiss]);
}
