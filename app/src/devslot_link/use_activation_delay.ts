import { useEffect, useState } from 'react';

/** How long a confirmation ignores activation after it appears or the window regains focus. */
export const ACTIVATION_DELAY_MS = 1000;

/**
 * Whether the confirmation may be activated. It is not for a moment after it
 * mounts and each time the window regains focus, so a click or key press that
 * was aimed at something else (a double click, a page that moves under the
 * cursor) cannot confirm a request the user has not read.
 */
export function useActivationDelay(): boolean {
	const [isArmed, setIsArmed] = useState(false);

	useEffect(() => {
		let timeoutId: ReturnType<typeof setTimeout> | undefined;
		const arm = () => {
			clearTimeout(timeoutId);
			setIsArmed(false);
			timeoutId = setTimeout(() => setIsArmed(true), ACTIVATION_DELAY_MS);
		};
		arm();
		window.addEventListener('focus', arm);
		return () => {
			clearTimeout(timeoutId);
			window.removeEventListener('focus', arm);
		};
	}, []);

	return isArmed;
}
