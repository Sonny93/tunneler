import type { ImportCandidateDto } from '#/daemon/protocol';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const UNKNOWN_HOST_LABEL = 'hôte à choisir';
const PARTS_SEPARATOR = ' · ';

/** The endpoint, the source line and the conflict note of a candidate, as one line of text. */
export function describeImportCandidate(
	candidate: Readonly<ImportCandidateDto>,
	fallbackHost: string,
	conflictNote: string | null
): string {
	const endpoint = getTunnelAccessibleEndpointLabel({
		...candidate,
		host: candidate.host ?? (fallbackHost || UNKNOWN_HOST_LABEL),
	});
	const parts = [
		endpoint,
		candidate.line === undefined ? null : `ligne ${candidate.line}`,
		conflictNote,
	];
	return parts.filter((part) => part !== null).join(PARTS_SEPARATOR);
}
