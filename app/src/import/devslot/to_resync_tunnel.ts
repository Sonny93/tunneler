import type {
	DevslotResyncTunnelDto,
	DevslotStaleTunnelDto,
} from '#/daemon/protocol';

/** The tunnel exactly as the discovery returned it: the daemon removes it only if it still matches. */
export function toResyncTunnel({
	tunnel,
	isTemporary,
}: Readonly<DevslotStaleTunnelDto>): DevslotResyncTunnelDto {
	return { id: tunnel.id, definition: tunnel, isTemporary };
}
