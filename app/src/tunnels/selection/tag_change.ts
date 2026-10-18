import { toTunnelSpec } from '#/tunnel_form/to_tunnel_spec';
import { toTunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import type { TunnelDefinitionDto, TunnelDto } from '#/daemon/protocol';

export type TagChange =
	| { kind: 'add'; tags: readonly string[] }
	| { kind: 'remove'; tag: string };

const TAGS_SEPARATOR = ', ';

export function applyTagChange(
	tags: readonly string[],
	change: Readonly<TagChange>
): string[] {
	if (change.kind === 'add') return [...new Set([...tags, ...change.tags])];
	return tags.filter((tag) => tag !== change.tag);
}

/** The tunnels whose tags the change would modify. */
export function findTunnelsToRetag(
	tunnels: readonly TunnelDto[],
	change: Readonly<TagChange>
): TunnelDto[] {
	return tunnels.filter(
		({ tags }) => applyTagChange(tags, change).length !== tags.length
	);
}

/** The definition the daemon expects, built from the tunnel the way the edit form builds it. */
export function toRetaggedDefinition(
	tunnel: Readonly<TunnelDto>,
	change: Readonly<TagChange>
): TunnelDefinitionDto {
	const tags = applyTagChange(tunnel.tags, change).join(TAGS_SEPARATOR);
	return {
		id: tunnel.id,
		...toTunnelSpec({ ...toTunnelFormValues(tunnel), tags }),
	};
}
