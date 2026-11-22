import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { ImportConflictDto, TunnelDto } from '#/daemon/protocol';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

/** Names the tunnel holding the id by its endpoint, or says "another tunnel" when it is no longer in `tunnels`. */
export function describeTunnelHolder(
	holderId: string,
	tunnels: readonly Readonly<TunnelDto>[],
	i18n: I18n
): string {
	const holder = tunnels.find((tunnel) => tunnel.id === holderId);
	return holder === undefined
		? i18n._(msg`another tunnel`)
		: getTunnelEndpointLabel(holder);
}

export function describeImportConflict(
	conflict: Readonly<ImportConflictDto>,
	tunnels: readonly Readonly<TunnelDto>[],
	i18n: I18n
): string {
	switch (conflict.kind) {
		case 'alreadyConfigured':
			return i18n._(msg`Already configured`);
		case 'listenConflict': {
			const tunnelLabel = describeTunnelHolder(
				conflict.holderId,
				tunnels,
				i18n
			);
			return i18n._(msg`Port already used by ${tunnelLabel}`);
		}
		case 'duplicateInImport':
			return i18n._(msg`Same port as a tunnel above`);
	}
}
