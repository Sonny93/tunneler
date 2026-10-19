import type { ImportCandidateDto, TunnelSpecDto } from '#/daemon/protocol';

export type ImportSpec = { index: number; spec: TunnelSpecDto };

type BuildImportSpecsInput = {
	candidates: readonly Readonly<ImportCandidateDto>[];
	selectedIndexes: ReadonlySet<number>;
	fallbackHost: string;
	tags: readonly string[];
};

function buildSpec(
	candidate: Readonly<ImportCandidateDto>,
	fallbackHost: string,
	tags: readonly string[]
): TunnelSpecDto {
	const hasTarget = candidate.kind !== 'dynamic';
	const isTargetPortDefault =
		candidate.targetPort === undefined ||
		candidate.targetPort === candidate.listenPort;
	return {
		host: candidate.host ?? fallbackHost,
		kind: candidate.kind,
		listenPort: candidate.listenPort,
		...(candidate.listenAddress !== undefined && {
			listenAddress: candidate.listenAddress,
		}),
		...(hasTarget &&
			candidate.targetHost !== undefined && {
				targetHost: candidate.targetHost,
			}),
		...(hasTarget &&
			!isTargetPortDefault && { targetPort: candidate.targetPort }),
		tags: [...tags],
	};
}

/** The specs of the selected candidates, in candidate order, each with the index of its candidate. */
export function buildImportSpecs({
	candidates,
	selectedIndexes,
	fallbackHost,
	tags,
}: Readonly<BuildImportSpecsInput>): ImportSpec[] {
	return candidates.flatMap((candidate, index) => {
		if (!selectedIndexes.has(index)) return [];
		return [{ index, spec: buildSpec(candidate, fallbackHost, tags) }];
	});
}
