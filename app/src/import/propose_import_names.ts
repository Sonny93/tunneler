import type { ImportCandidateDto } from '#/daemon/protocol';

const FALLBACK_HOST_LABEL = 'ssh';
const FIRST_SUFFIX = 2;

type NameSource = Pick<ImportCandidateDto, 'label' | 'host' | 'listenPort'>;

function buildBaseName(candidate: Readonly<NameSource>): string {
	return (
		candidate.label ??
		`${candidate.host ?? FALLBACK_HOST_LABEL} ${candidate.listenPort}`
	);
}

function makeUnique(baseName: string, takenNames: ReadonlySet<string>): string {
	let name = baseName;
	for (
		let suffix = FIRST_SUFFIX;
		takenNames.has(name.toLowerCase());
		suffix += 1
	) {
		name = `${baseName} ${suffix}`;
	}
	return name;
}

/**
 * One name per candidate: its label, or `host port`. Names are unique ignoring
 * case against the existing tunnels and the candidates before them, with a
 * numeric suffix when needed.
 */
export function proposeImportNames(
	candidates: readonly Readonly<NameSource>[],
	existingNames: readonly string[]
): string[] {
	const takenNames = new Set(existingNames.map((name) => name.toLowerCase()));
	return candidates.map((candidate) => {
		const name = makeUnique(buildBaseName(candidate), takenNames);
		takenNames.add(name.toLowerCase());
		return name;
	});
}
