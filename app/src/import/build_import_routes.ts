import type { ImportCandidateDto, ImportRouteDto } from '#/daemon/protocol';

export type ImportRoutes = {
	routes: ImportRouteDto[];
	/** `candidateIndexes[n]` is the index in the candidates of `routes[n]`. */
	candidateIndexes: number[];
};

function buildRoute(
	candidate: Readonly<ImportCandidateDto>,
	host: string
): ImportRouteDto {
	return {
		host,
		kind: candidate.kind,
		listenPort: candidate.listenPort,
		...(candidate.listenAddress === undefined
			? {}
			: { listenAddress: candidate.listenAddress }),
		...(candidate.targetHost === undefined
			? {}
			: { targetHost: candidate.targetHost }),
		...(candidate.targetPort === undefined
			? {}
			: { targetPort: candidate.targetPort }),
	};
}

/**
 * The routes to classify: each candidate with its own host or, without one,
 * the default host. A candidate with neither has no route yet.
 */
export function buildImportRoutes(
	candidates: readonly Readonly<ImportCandidateDto>[],
	defaultHost: string
): ImportRoutes {
	const routes: ImportRouteDto[] = [];
	const candidateIndexes: number[] = [];
	candidates.forEach((candidate, index) => {
		const host = candidate.host ?? defaultHost;
		if (host === '') return;
		routes.push(buildRoute(candidate, host));
		candidateIndexes.push(index);
	});
	return { routes, candidateIndexes };
}
