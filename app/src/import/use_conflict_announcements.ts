import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useRef } from 'react';

import { describeConflictCount } from '#/import/import_texts';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';

export const CONFLICT_CHECK_FAILED_MESSAGE = msg`Could not check for conflicts; creation will still refuse clashes.`;

type UseConflictAnnouncementsReturn = {
	/** Announces the new number of conflicts when it differs from the previous answer; the first answer is silent. */
	reportCount: (count: number) => void;
	reportFailure: () => void;
};

/** The polite announcements that follow a new classification, so a changed result is not silent. */
export function useConflictAnnouncements(): UseConflictAnnouncementsReturn {
	const { i18n } = useLingui();
	const lastCount = useRef<number | null>(null);

	const reportCount = (count: number) => {
		const previous = lastCount.current;
		lastCount.current = count;
		if (previous === null || previous === count) return;
		useAnnouncerStore
			.getState()
			.announce(describeConflictCount(count, i18n), 'polite');
	};
	const reportFailure = () =>
		useAnnouncerStore
			.getState()
			.announce(i18n._(CONFLICT_CHECK_FAILED_MESSAGE), 'polite');

	return { reportCount, reportFailure };
}
