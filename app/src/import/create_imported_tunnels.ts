import type { I18n } from '@lingui/core';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import type { ImportSpec } from '#/import/build_import_specs';
import {
	getEnglishTexts,
	getSubmissionErrorMessages,
} from '#/shared/submission_error_messages';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

export type ImportFailure = {
	index: number;
	label: string;
	messages: string[];
	/** The parts of `messages` that are raw English from the daemon. */
	englishTexts: string[];
};

export type ImportOutcome = {
	createdIndexes: number[];
	failures: ImportFailure[];
};

/** Creates the tunnels one after the other; a failure does not stop the next ones. */
export async function createImportedTunnels(
	gateway: DaemonGateway,
	importSpecs: readonly ImportSpec[],
	onProgress: (doneCount: number) => void,
	i18n: I18n
): Promise<ImportOutcome> {
	const createdIndexes: number[] = [];
	const failures: ImportFailure[] = [];
	for (const [position, { index, spec }] of importSpecs.entries()) {
		try {
			// oxlint-disable-next-line no-await-in-loop -- tunnels are created in order so progress and port conflicts stay predictable
			await gateway.createTunnel(spec);
			createdIndexes.push(index);
		} catch (rejection) {
			const error = toDaemonCommandError(rejection);
			failures.push({
				index,
				label: getTunnelEndpointLabel(spec),
				messages: getSubmissionErrorMessages(error, i18n),
				englishTexts: getEnglishTexts(error, i18n),
			});
		}
		onProgress(position + 1);
	}
	return { createdIndexes, failures };
}
