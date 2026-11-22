import { useEffect, useEffectEvent, useRef, useState } from 'react';

import type { ImportCandidateDto, ImportConflictDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { buildImportRoutes } from '#/import/build_import_routes';
import { useConflictAnnouncements } from '#/import/use_conflict_announcements';

export type ImportConflicts = (ImportConflictDto | null)[];

type UseImportConflictsReturn = {
	/** One per candidate; the previous answer stays in place while a new one is awaited. */
	conflicts: ImportConflicts;
	/** `false` until the daemon answered once, so the form can wait for it before it offers to import. */
	isClassified: boolean;
	/** `true` while the last check failed: `conflicts` is then the last good answer. */
	hasFailed: boolean;
};

function spreadConflicts(
	candidateCount: number,
	candidateIndexes: readonly number[],
	answered: readonly (ImportConflictDto | null)[]
): ImportConflicts {
	const conflicts: ImportConflicts = Array.from(
		{ length: candidateCount },
		() => null
	);
	candidateIndexes.forEach((candidateIndex, position) => {
		conflicts[candidateIndex] = answered[position] ?? null;
	});
	return conflicts;
}

/**
 * Asks the daemon, on mount and whenever `defaultHost` changes, what stops each
 * candidate from being imported. `onFirstAnswer` runs once with the first
 * answer, to preselect.
 */
export function useImportConflicts(
	candidates: readonly Readonly<ImportCandidateDto>[],
	defaultHost: string,
	onFirstAnswer: (conflicts: ImportConflicts) => void
): UseImportConflictsReturn {
	const gateway = useDaemonGateway();
	const [conflicts, setConflicts] = useState<ImportConflicts>(() =>
		Array.from({ length: candidates.length }, () => null)
	);
	const [isClassified, setIsClassified] = useState(false);
	const [hasFailed, setHasFailed] = useState(false);
	const announcements = useConflictAnnouncements();
	const hasAnswered = useRef(false);
	const handleFirstAnswer = useEffectEvent(onFirstAnswer);
	const reportCount = useEffectEvent(announcements.reportCount);
	const reportFailure = useEffectEvent(announcements.reportFailure);

	useEffect(() => {
		let isCurrent = true;
		const { routes, candidateIndexes } = buildImportRoutes(
			candidates,
			defaultHost
		);
		const settle = (answered: readonly (ImportConflictDto | null)[]) => {
			if (!isCurrent) return;
			const next = spreadConflicts(
				candidates.length,
				candidateIndexes,
				answered
			);
			setConflicts(next);
			setIsClassified(true);
			setHasFailed(false);
			reportCount(next.filter((conflict) => conflict !== null).length);
			if (hasAnswered.current) return;
			hasAnswered.current = true;
			handleFirstAnswer(next);
		};
		const load = routes.length === 0 ? [] : gateway.classifyImports(routes);
		const keepPrevious = () => {
			if (!isCurrent) return;
			setIsClassified(true);
			setHasFailed(true);
			reportFailure();
		};
		void Promise.resolve(load).then(settle, keepPrevious);
		return () => {
			isCurrent = false;
		};
	}, [candidates, defaultHost, gateway]);

	return { conflicts, isClassified, hasFailed };
}
