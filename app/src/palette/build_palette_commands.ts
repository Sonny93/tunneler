import { summarizeTags } from '#/sidebar/summarize_tags';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import type { PaletteCommand } from '#/palette/palette_command';
import type { AvailableOpenerDto, TunnelDto } from '#/daemon/protocol';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';
import {
	describeTunnelOpenCommand,
	listAlternativeChoices,
	toDefaultChoice,
} from '#/openers/open_choices';

type PaletteSources = {
	tunnels: readonly Readonly<TunnelDto>[];
	hosts: readonly string[];
	openers: readonly Readonly<AvailableOpenerDto>[];
};

const GENERAL_COMMANDS: readonly PaletteCommand[] = [
	{
		id: 'general:new-tunnel',
		label: 'Nouveau tunnel',
		category: 'Général',
		action: { kind: 'newTunnel' },
	},
	{
		id: 'general:new-temporary-tunnel',
		label: 'Nouveau tunnel temporaire…',
		category: 'Général',
		action: { kind: 'newTemporaryTunnel' },
	},
	{
		id: 'general:import',
		label: 'Importer des tunnels',
		category: 'Général',
		action: { kind: 'importTunnels' },
	},
	{
		id: 'general:settings',
		label: 'Ouvrir les paramètres',
		category: 'Général',
		action: { kind: 'openSettings' },
	},
	{
		id: 'general:reveal-config',
		label: 'Ouvrir le dossier de configuration',
		category: 'Général',
		action: { kind: 'revealConfig' },
	},
	{
		id: 'general:sidebar',
		label: 'Masquer ou afficher le panneau',
		category: 'Général',
		action: { kind: 'toggleSidebar' },
	},
	{
		id: 'general:shortcuts',
		label: 'Afficher les raccourcis clavier',
		category: 'Général',
		action: { kind: 'showShortcuts' },
	},
];

function buildToggleCommand(tunnel: Readonly<TunnelDto>): PaletteCommand {
	const tunnelId = tunnel.id;
	return isTunnelRunning(tunnel.state)
		? {
				id: `tunnel:${tunnelId}:stop`,
				label: `Arrêter ${getTunnelEndpointLabel(tunnel)}`,
				category: 'Tunnel',
				action: { kind: 'stopTunnel', tunnelId },
			}
		: {
				id: `tunnel:${tunnelId}:start`,
				label: `Démarrer ${getTunnelEndpointLabel(tunnel)}`,
				category: 'Tunnel',
				action: { kind: 'startTunnel', tunnelId },
			};
}

function buildStandbyCommands(tunnel: Readonly<TunnelDto>): PaletteCommand[] {
	if (!tunnel.isAwake) return [];
	return [
		{
			id: `tunnel:${tunnel.id}:standby`,
			label: `Mettre en veille ${getTunnelEndpointLabel(tunnel)}`,
			category: 'Tunnel',
			action: { kind: 'standbyTunnel', tunnelId: tunnel.id },
		},
	];
}

function buildOpenCommands(
	tunnel: Readonly<TunnelDto>,
	openers: readonly Readonly<AvailableOpenerDto>[]
): PaletteCommand[] {
	if (tunnel.kind !== 'local') return [];
	const endpointLabel = getTunnelEndpointLabel(tunnel);
	const defaultChoice = toDefaultChoice(tunnel.defaultOpener);
	const alternatives = listAlternativeChoices(tunnel.defaultOpener, openers);
	return [
		{
			id: `tunnel:${tunnel.id}:open`,
			label: describeTunnelOpenCommand(endpointLabel, defaultChoice),
			category: 'Tunnel',
			action: { kind: 'openTunnel', id: tunnel.id },
		},
		...alternatives.map((choice) => ({
			id: `tunnel:${tunnel.id}:open:${choice.id}`,
			label: describeTunnelOpenCommand(endpointLabel, choice),
			category: 'Tunnel' as const,
			action: { kind: 'openTunnel' as const, id: tunnel.id, opener: choice.id },
		})),
	];
}

function buildTunnelCommands(
	tunnel: Readonly<TunnelDto>,
	openers: readonly Readonly<AvailableOpenerDto>[]
): PaletteCommand[] {
	const tunnelId = tunnel.id;
	return [
		buildToggleCommand(tunnel),
		...buildStandbyCommands(tunnel),
		{
			id: `tunnel:${tunnelId}:details`,
			label: `Détails de ${getTunnelEndpointLabel(tunnel)}`,
			category: 'Tunnel',
			action: { kind: 'showTunnelDetails', tunnelId },
		},
		{
			id: `tunnel:${tunnelId}:edit`,
			label: `Modifier ${getTunnelEndpointLabel(tunnel)}`,
			category: 'Tunnel',
			action: { kind: 'editTunnel', tunnelId },
		},
		...buildOpenCommands(tunnel, openers),
	];
}

function hasAwakeTunnel(
	tunnels: readonly Readonly<TunnelDto>[],
	tag: string
): boolean {
	return tunnels.some((tunnel) => tunnel.isAwake && tunnel.tags.includes(tag));
}

function buildTagCommands(
	tunnels: readonly Readonly<TunnelDto>[]
): PaletteCommand[] {
	return summarizeTags(tunnels).flatMap(({ tag, started, total }) => {
		const commands: PaletteCommand[] = [];
		if (started < total) {
			commands.push({
				id: `tag:${tag}:start`,
				label: `Démarrer le tag ${tag}`,
				category: 'Tag',
				action: { kind: 'startTag', tag },
			});
		}
		if (started > 0) {
			commands.push({
				id: `tag:${tag}:stop`,
				label: `Arrêter le tag ${tag}`,
				category: 'Tag',
				action: { kind: 'stopTag', tag },
			});
		}
		if (hasAwakeTunnel(tunnels, tag)) {
			commands.push({
				id: `tag:${tag}:standby`,
				label: `Mettre en veille le tag ${tag}`,
				category: 'Tag',
				action: { kind: 'standbyTag', tag },
			});
		}
		commands.push({
			id: `tag:${tag}:rename`,
			label: `Renommer le tag ${tag}…`,
			category: 'Tag',
			action: { kind: 'renameTag', tag },
		});
		commands.push({
			id: `tag:${tag}:delete`,
			label: `Supprimer le tag ${tag}…`,
			category: 'Tag',
			action: { kind: 'deleteTag', tag },
		});
		return commands;
	});
}

function listHosts({
	tunnels,
	hosts,
}: Readonly<PaletteSources>): readonly string[] {
	const allHosts = new Set([...hosts, ...tunnels.map((tunnel) => tunnel.host)]);
	return [...allHosts].sort(compareTextCaseInsensitively);
}

function buildHostCommands(host: string): PaletteCommand[] {
	return [
		{
			id: `host:${host}:config`,
			label: `Configuration de ${host}`,
			category: 'Hôte',
			action: { kind: 'showHostConfig', host },
		},
		{
			id: `host:${host}:x11`,
			label: `X11 pour ${host}`,
			category: 'Hôte',
			action: { kind: 'showX11', host },
		},
	];
}

function buildStandbyAllCommands(
	tunnels: readonly Readonly<TunnelDto>[]
): PaletteCommand[] {
	if (listAwakeTunnels(tunnels).length === 0) return [];
	return [
		{
			id: 'general:standby-all',
			label: 'Mettre en veille les tunnels réveillés',
			category: 'Général',
			action: { kind: 'standbyAll' },
		},
	];
}

/** Every command the palette can run, in display order: general, tunnels, tags, hosts. */
export function buildPaletteCommands(
	sources: Readonly<PaletteSources>
): PaletteCommand[] {
	return [
		...GENERAL_COMMANDS,
		...buildStandbyAllCommands(sources.tunnels),
		...sources.tunnels.flatMap((tunnel) =>
			buildTunnelCommands(tunnel, sources.openers)
		),
		...buildTagCommands(sources.tunnels),
		...listHosts(sources).flatMap(buildHostCommands),
	];
}
