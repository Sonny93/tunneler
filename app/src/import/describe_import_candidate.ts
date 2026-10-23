import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { ImportCandidateDto } from '#/daemon/protocol';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const PARTS_SEPARATOR = ' · ';

/** The endpoint of a candidate in words, which stands for its name: `3333 to nova:3333`. */
export function describeImportCandidateLabel(
	candidate: Readonly<ImportCandidateDto>,
	fallbackHost: string,
	i18n: I18n
): string {
	return getTunnelAccessibleEndpointLabel(
		{
			...candidate,
			host: candidate.host ?? (fallbackHost || i18n._(msg`host to choose`)),
		},
		i18n
	);
}

/** The comment, the source line and the conflict note of a candidate, as one line of text. */
export function describeImportCandidateDetails(
	candidate: Readonly<ImportCandidateDto>,
	conflictNote: string | null,
	i18n: I18n
): string {
	const { line } = candidate;
	const parts = [
		candidate.comment ?? null,
		line === undefined ? null : i18n._(msg`line ${line}`),
		conflictNote,
	];
	return parts.filter((part) => part !== null).join(PARTS_SEPARATOR);
}
