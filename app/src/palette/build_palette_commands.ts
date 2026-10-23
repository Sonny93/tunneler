import type { I18n, MessageDescriptor } from '@lingui/core';
import { msg, t } from '@lingui/core/macro';

import type { AvailableOpenerDto, TunnelDto } from '#/daemon/protocol';
import {
	describeTunnelOpenCommand,
	listAlternativeChoices,
	toDefaultChoice,
} from '#/openers/open_choices';
import type { PaletteCommand } from '#/palette/palette_command';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type PaletteSources = {
	tunnels: readonly Readonly<TunnelDto>[];
	hosts: readonly string[];
	openers: readonly Readonly<AvailableOpenerDto>[];
};

type CategoryKey = 'general' | 'tunnel' | 'tag' | 'host';

const CATEGORY_LABELS: Readonly<Record<CategoryKey, MessageDescriptor>> = {
	general: msg`General`,
	tunnel: msg`Tunnel`,
	tag: msg`Tag`,
	host: msg`Host`,
};

function buildGeneralCommands(i18n: I18n): PaletteCommand[] {
	const category = i18n._(CATEGORY_LABELS.general);
	return [
		{
			id: 'general:new-tunnel',
			label: t(i18n)`New tunnel`,
			category,
			action: { kind: 'newTunnel' },
		},
		{
			id: 'general:new-temporary-tunnel',
			label: t(i18n)`New temporary tunnel…`,
			category,
			action: { kind: 'newTemporaryTunnel' },
		},
		{
			id: 'general:import',
			label: t(i18n)`Import tunnels`,
			category,
			action: { kind: 'importTunnels' },
		},
		{
			id: 'general:settings',
			label: t(i18n)`Open settings`,
			category,
			action: { kind: 'openSettings' },
		},
		{
			id: 'general:reveal-config',
			label: t(i18n)`Open the configuration folder`,
			category,
			action: { kind: 'revealConfig' },
		},
		{
			id: 'general:sidebar',
			label: t(i18n)`Hide or show the sidebar`,
			category,
			action: { kind: 'toggleSidebar' },
		},
		{
			id: 'general:shortcuts',
			label: t(i18n)`Show keyboard shortcuts`,
			category,
			action: { kind: 'showShortcuts' },
		},
		{
			id: 'general:documentation',
			label: t(i18n)`Open the documentation`,
			category,
			action: { kind: 'openDocumentation' },
		},
	];
}

function buildToggleCommand(
	tunnel: Readonly<TunnelDto>,
	i18n: I18n
): PaletteCommand {
	const tunnelId = tunnel.id;
	const endpointLabel = getTunnelEndpointLabel(tunnel);
	const category = i18n._(CATEGORY_LABELS.tunnel);
	return isTunnelRunning(tunnel.state)
		? {
				id: `tunnel:${tunnelId}:stop`,
				label: t(i18n)`Stop ${endpointLabel}`,
				category,
				action: { kind: 'stopTunnel', tunnelId },
			}
		: {
				id: `tunnel:${tunnelId}:start`,
				label: t(i18n)`Start ${endpointLabel}`,
				category,
				action: { kind: 'startTunnel', tunnelId },
			};
}

function buildStandbyCommands(
	tunnel: Readonly<TunnelDto>,
	i18n: I18n
): PaletteCommand[] {
	if (!tunnel.isAwake) return [];
	const endpointLabel = getTunnelEndpointLabel(tunnel);
	return [
		{
			id: `tunnel:${tunnel.id}:standby`,
			label: t(i18n)`Send ${endpointLabel} to standby`,
			category: i18n._(CATEGORY_LABELS.tunnel),
			action: { kind: 'standbyTunnel', tunnelId: tunnel.id },
		},
	];
}

function buildOpenCommands(
	tunnel: Readonly<TunnelDto>,
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): PaletteCommand[] {
	if (tunnel.kind !== 'local') return [];
	const category = i18n._(CATEGORY_LABELS.tunnel);
	const endpointLabel = getTunnelEndpointLabel(tunnel);
	const defaultChoice = toDefaultChoice(tunnel.defaultOpener, i18n);
	const alternatives = listAlternativeChoices(
		tunnel.defaultOpener,
		openers,
		i18n
	);
	return [
		{
			id: `tunnel:${tunnel.id}:open`,
			label: describeTunnelOpenCommand(endpointLabel, defaultChoice, i18n),
			category,
			action: { kind: 'openTunnel', id: tunnel.id },
		},
		...alternatives.map((choice) => ({
			id: `tunnel:${tunnel.id}:open:${choice.id}`,
			label: describeTunnelOpenCommand(endpointLabel, choice, i18n),
			category,
			action: { kind: 'openTunnel' as const, id: tunnel.id, opener: choice.id },
		})),
	];
}

function buildTunnelCommands(
	tunnel: Readonly<TunnelDto>,
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): PaletteCommand[] {
	const tunnelId = tunnel.id;
	const endpointLabel = getTunnelEndpointLabel(tunnel);
	const category = i18n._(CATEGORY_LABELS.tunnel);
	return [
		buildToggleCommand(tunnel, i18n),
		...buildStandbyCommands(tunnel, i18n),
		{
			id: `tunnel:${tunnelId}:details`,
			label: t(i18n)`Details of ${endpointLabel}`,
			category,
			action: { kind: 'showTunnelDetails', tunnelId },
		},
		{
			id: `tunnel:${tunnelId}:edit`,
			label: t(i18n)`Edit ${endpointLabel}`,
			category,
			action: { kind: 'editTunnel', tunnelId },
		},
		...buildOpenCommands(tunnel, openers, i18n),
	];
}

function hasAwakeTunnel(
	tunnels: readonly Readonly<TunnelDto>[],
	tag: string
): boolean {
	return tunnels.some((tunnel) => tunnel.isAwake && tunnel.tags.includes(tag));
}

function buildTagCommands(
	tunnels: readonly Readonly<TunnelDto>[],
	i18n: I18n
): PaletteCommand[] {
	const category = i18n._(CATEGORY_LABELS.tag);
	return summarizeTags(tunnels).flatMap(({ tag, started, total }) => {
		const commands: PaletteCommand[] = [];
		if (started < total) {
			commands.push({
				id: `tag:${tag}:start`,
				label: t(i18n)`Start tag ${tag}`,
				category,
				action: { kind: 'startTag', tag },
			});
		}
		if (started > 0) {
			commands.push({
				id: `tag:${tag}:stop`,
				label: t(i18n)`Stop tag ${tag}`,
				category,
				action: { kind: 'stopTag', tag },
			});
		}
		if (hasAwakeTunnel(tunnels, tag)) {
			commands.push({
				id: `tag:${tag}:standby`,
				label: t(i18n)`Send tag ${tag} to standby`,
				category,
				action: { kind: 'standbyTag', tag },
			});
		}
		commands.push({
			id: `tag:${tag}:rename`,
			label: t(i18n)`Rename tag ${tag}…`,
			category,
			action: { kind: 'renameTag', tag },
		});
		commands.push({
			id: `tag:${tag}:delete`,
			label: t(i18n)`Delete tag ${tag}…`,
			category,
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

function buildHostCommands(host: string, i18n: I18n): PaletteCommand[] {
	const category = i18n._(CATEGORY_LABELS.host);
	return [
		{
			id: `host:${host}:config`,
			label: t(i18n)`Configuration of ${host}`,
			category,
			action: { kind: 'showHostConfig', host },
		},
		{
			id: `host:${host}:x11`,
			label: t(i18n)`X11 for ${host}`,
			category,
			action: { kind: 'showX11', host },
		},
	];
}

function buildStandbyAllCommands(
	tunnels: readonly Readonly<TunnelDto>[],
	i18n: I18n
): PaletteCommand[] {
	if (listAwakeTunnels(tunnels).length === 0) return [];
	return [
		{
			id: 'general:standby-all',
			label: t(i18n)`Send awake tunnels to standby`,
			category: i18n._(CATEGORY_LABELS.general),
			action: { kind: 'standbyAll' },
		},
	];
}

/** Every command the palette can run, in display order: general, tunnels, tags, hosts. */
export function buildPaletteCommands(
	sources: Readonly<PaletteSources>,
	i18n: I18n
): PaletteCommand[] {
	return [
		...buildGeneralCommands(i18n),
		...buildStandbyAllCommands(sources.tunnels, i18n),
		...sources.tunnels.flatMap((tunnel) =>
			buildTunnelCommands(tunnel, sources.openers, i18n)
		),
		...buildTagCommands(sources.tunnels, i18n),
		...listHosts(sources).flatMap((host) => buildHostCommands(host, i18n)),
	];
}
