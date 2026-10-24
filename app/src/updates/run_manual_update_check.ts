import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { useUpdateStore } from '#/updates/update_store';

export type ManualUpdateCheckOutcome =
	| { kind: 'skipped' }
	| { kind: 'upToDate' }
	| { kind: 'available'; version: string }
	| { kind: 'failed' };

/**
 * Asks for a newer version whatever the `checkForUpdates` setting says. A
 * found offer is stored and its banner shown again; a failure leaves the
 * stored offer alone. Nothing runs while an installation is in progress.
 */
export async function runManualUpdateCheck(
	gateway: DaemonGateway
): Promise<ManualUpdateCheckOutcome> {
	if (useUpdateStore.getState().isInstalling) return { kind: 'skipped' };
	try {
		const offer = await gateway.checkForUpdate();
		if (offer === null) return { kind: 'upToDate' };
		useUpdateStore.getState().showOffer(offer);
		return { kind: 'available', version: offer.version };
	} catch {
		return { kind: 'failed' };
	}
}
