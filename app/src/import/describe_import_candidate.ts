import type { ImportCandidateDto } from '#/daemon/protocol';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const UNKNOWN_HOST_LABEL = 'hôte à choisir';
const PARTS_SEPARATOR = ' · ';

/** The endpoint of a candidate in words, which stands for its name: `3333 vers nova:3333`. */
export function describeImportCandidateLabel(
	candidate: Readonly<ImportCandidateDto>,
	fallbackHost: string
): string {
	return getTunnelAccessibleEndpointLabel({
		...candidate,
		host: candidate.host ?? (fallbackHost || UNKNOWN_HOST_LABEL),
	});
}

/** The comment, the source line and the conflict note of a candidate, as one line of text. */
export function describeImportCandidateDetails(
	candidate: Readonly<ImportCandidateDto>,
	conflictNote: string | null
): string {
	const parts = [
		candidate.comment ?? null,
		candidate.line === undefined ? null : `ligne ${candidate.line}`,
		conflictNote,
	];
	return parts.filter((part) => part !== null).join(PARTS_SEPARATOR);
}
