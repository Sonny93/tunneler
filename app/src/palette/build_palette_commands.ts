import { summarizeTags } from '#/sidebar/summarize_tags';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import type { SettingsDto, TunnelDto } from '#/daemon/protocol';
import type { PaletteCommand } from '#/palette/palette_command';
import { getBrowserTarget } from '#/tunnel_details/browser_target';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

type PaletteSources = {
	tunnels: readonly Readonly<TunnelDto>[];
	hosts: readonly string[];
	settings: Readonly<SettingsDto> | null;
};

const GENERAL_COMMANDS: readonly PaletteCommand[] = [
	{
		id: 'general:new-tunnel',
		label: 'Nouveau tunnel',
		category: 'Général',
		action: { kind: 'newTunnel' },
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
				label: `Arrêter ${tunnel.name}`,
				category: 'Tunnel',
				action: { kind: 'stopTunnel', tunnelId },
			}
		: {
				id: `tunnel:${tunnelId}:start`,
				label: `Démarrer ${tunnel.name}`,
				category: 'Tunnel',
				action: { kind: 'startTunnel', tunnelId },
			};
}

function buildBrowserCommands(
	tunnel: Readonly<TunnelDto>,
	settings: Readonly<SettingsDto> | null
): PaletteCommand[] {
	const target = getBrowserTarget(tunnel, settings?.defaultBindAddress);
	if (target === null || !isTunnelRunning(tunnel.state)) return [];
	return [
		{
			id: `tunnel:${tunnel.id}:browser`,
			label: `Ouvrir ${tunnel.name} dans le navigateur`,
			category: 'Tunnel',
			action: { kind: 'openInBrowser', ...target },
		},
	];
}

function buildTunnelCommands(
	tunnel: Readonly<TunnelDto>,
	settings: Readonly<SettingsDto> | null
): PaletteCommand[] {
	const tunnelId = tunnel.id;
	return [
		buildToggleCommand(tunnel),
		{
			id: `tunnel:${tunnelId}:details`,
			label: `Détails de ${tunnel.name}`,
			category: 'Tunnel',
			action: { kind: 'showTunnelDetails', tunnelId },
		},
		{
			id: `tunnel:${tunnelId}:edit`,
			label: `Modifier ${tunnel.name}`,
			category: 'Tunnel',
			action: { kind: 'editTunnel', tunnelId },
		},
		...buildBrowserCommands(tunnel, settings),
	];
}

function buildTagCommands(
	tunnels: readonly Readonly<TunnelDto>[]
): PaletteCommand[] {
	return summarizeTags(tunnels).flatMap(({ tag, running, total }) => {
		const commands: PaletteCommand[] = [];
		if (running < total) {
			commands.push({
				id: `tag:${tag}:start`,
				label: `Démarrer le tag ${tag}`,
				category: 'Tag',
				action: { kind: 'startTag', tag },
			});
		}
		if (running > 0) {
			commands.push({
				id: `tag:${tag}:stop`,
				label: `Arrêter le tag ${tag}`,
				category: 'Tag',
				action: { kind: 'stopTag', tag },
			});
		}
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

/** Every command the palette can run, in display order: general, tunnels, tags, hosts. */
export function buildPaletteCommands(
	sources: Readonly<PaletteSources>
): PaletteCommand[] {
	return [
		...GENERAL_COMMANDS,
		...sources.tunnels.flatMap((tunnel) =>
			buildTunnelCommands(tunnel, sources.settings)
		),
		...buildTagCommands(sources.tunnels),
		...listHosts(sources).flatMap(buildHostCommands),
	];
}
