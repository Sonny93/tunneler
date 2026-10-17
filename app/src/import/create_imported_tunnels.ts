import type { DaemonGateway } from '#/daemon/daemon_gateway';
import type { ImportSpec } from '#/import/build_import_specs';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';

export type ImportFailure = { index: number; name: string; messages: string[] };

export type ImportOutcome = {
	createdIndexes: number[];
	failures: ImportFailure[];
};

/** Creates the tunnels one after the other; a failure does not stop the next ones. */
export async function createImportedTunnels(
	gateway: DaemonGateway,
	importSpecs: readonly ImportSpec[],
	onProgress: (doneCount: number) => void
): Promise<ImportOutcome> {
	const createdIndexes: number[] = [];
	const failures: ImportFailure[] = [];
	for (const [position, { index, spec }] of importSpecs.entries()) {
		try {
			await gateway.createTunnel(spec);
			createdIndexes.push(index);
		} catch (rejection) {
			const messages = getSubmissionErrorMessages(
				toDaemonCommandError(rejection)
			);
			failures.push({ index, name: spec.name, messages });
		}
		onProgress(position + 1);
	}
	return { createdIndexes, failures };
}
