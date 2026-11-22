import { useEffect, useEffectEvent, useState } from 'react';

import type { ImportConflictDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { DevslotClassifyRequest } from '#/import/devslot/find_devslot_problems';
import { useConflictAnnouncements } from '#/import/use_conflict_announcements';

type ConflictsByKey = ReadonlyMap<string, ImportConflictDto | null>;

type Answered = {
	signature: string;
	conflictsByKey: ConflictsByKey;
	hasFailed: boolean;
};

type UseDevslotConflictsReturn = {
	/** The last answer, by port: it stays in place while a new one is awaited. */
	conflictsByKey: ConflictsByKey;
	/** `true` while the answer for the current choice of ports has not arrived. */
	isPending: boolean;
	/** `true` while the last check failed: `conflictsByKey` is then the last good answer. */
	hasFailed: boolean;
};

const NO_CONFLICTS: ConflictsByKey = new Map();

function spreadAnswer(
	keys: readonly string[],
	conflicts: readonly (ImportConflictDto | null)[]
): ConflictsByKey {
	return new Map(keys.map((key, index) => [key, conflicts[index] ?? null]));
}

/**
 * Asks the daemon what each port clashes with, again whenever the request
 * changes; an answer to an older request is ignored. `onAnswer` runs with the
 * chosen ports and the conflicts of each answer that is kept.
 */
export function useDevslotConflicts(
	request: Readonly<DevslotClassifyRequest>,
	onAnswer: (chosenKeys: readonly string[], conflicts: ConflictsByKey) => void
): UseDevslotConflictsReturn {
	const gateway = useDaemonGateway();
	const announcements = useConflictAnnouncements();
	const [answered, setAnswered] = useState<Answered | null>(null);
	const signature = JSON.stringify([request.routes, request.keys]);
	const classify = useEffectEvent(() =>
		request.routes.length === 0
			? Promise.resolve([])
			: gateway.classifyImports(request.routes)
	);
	const handleAnswer = useEffectEvent((conflictsByKey: ConflictsByKey) =>
		onAnswer(request.chosenKeys, conflictsByKey)
	);
	const reportCount = useEffectEvent(announcements.reportCount);
	const reportFailure = useEffectEvent(announcements.reportFailure);
	const spread = useEffectEvent(
		(conflicts: Parameters<typeof spreadAnswer>[1]) =>
			spreadAnswer(request.keys, conflicts)
	);

	useEffect(() => {
		let isCurrent = true;
		const settle = (conflicts: Parameters<typeof spreadAnswer>[1]) => {
			if (!isCurrent) return;
			const conflictsByKey = spread(conflicts);
			setAnswered({ signature, conflictsByKey, hasFailed: false });
			reportCount(
				[...conflictsByKey.values()].filter((conflict) => conflict !== null)
					.length
			);
			handleAnswer(conflictsByKey);
		};
		const keepPrevious = () => {
			if (!isCurrent) return;
			setAnswered((previous) => ({
				signature,
				conflictsByKey: previous?.conflictsByKey ?? NO_CONFLICTS,
				hasFailed: true,
			}));
			reportFailure();
		};
		void classify().then(settle, keepPrevious);
		return () => {
			isCurrent = false;
		};
	}, [signature]);

	return {
		conflictsByKey: answered?.conflictsByKey ?? NO_CONFLICTS,
		isPending: answered?.signature !== signature,
		hasFailed: answered?.hasFailed ?? false,
	};
}
