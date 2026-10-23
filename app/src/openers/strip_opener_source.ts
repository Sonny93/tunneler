import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';

/** The part of an available opener that the daemon stores. */
export function stripOpenerSource(
	opener: Readonly<AvailableOpenerDto>
): OpenerSpecDto {
	const { id, name, command, args, ports } = opener;
	return { id, name, command, args, ports };
}
