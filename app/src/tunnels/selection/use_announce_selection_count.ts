import { useEffect, useRef } from 'react';
import { useLingui } from '@lingui/react/macro';

import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { describeSelectionCount } from '#/tunnels/selection/describe_selection_count';

/** Announces the new count politely whenever it changes, never on the first render. */
export function useAnnounceSelectionCount(count: number): void {
	const { i18n } = useLingui();
	const previousCountRef = useRef(count);

	useEffect(() => {
		if (previousCountRef.current === count) return;
		previousCountRef.current = count;
		useAnnouncerStore
			.getState()
			.announce(describeSelectionCount(count, i18n), 'polite');
	}, [count, i18n]);
}
