import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { DaemonGateway, Unsubscribe } from '#/daemon/daemon_gateway';
import type { EventMessage, UpdateProgressDto } from '#/daemon/protocol';

const EVENT_CHANNEL = 'daemon-event';
const DISCONNECTED_CHANNEL = 'daemon-disconnected';

async function call<TResult = void>(
	command: string,
	args?: Record<string, unknown>
): Promise<TResult> {
	try {
		return await invoke<TResult>(command, args);
	} catch (rejection) {
		throw toDaemonCommandError(rejection);
	}
}

export function createTauriGateway(): DaemonGateway {
	return {
		connect: () => call('connect_daemon'),
		snapshot: () => call('snapshot'),
		listHosts: () => call('list_hosts'),
		createTunnel: (spec) => call('create_tunnel', { spec }),
		updateTunnel: (definition) => call('update_tunnel', { definition }),
		startTemporaryTunnel: (spec) => call('start_temporary_tunnel', { spec }),
		persistTunnel: (id) => call('persist_tunnel', { id }),
		checkTarget: (id) => call('check_target', { id }),
		previewSound: (sound, event) => call('preview_sound', { sound, event }),
		deleteTunnel: (id) => call('delete_tunnel', { id }),
		startTunnel: (id) => call('start_tunnel', { id }),
		stopTunnel: (id) => call('stop_tunnel', { id }),
		restartTunnel: (id) => call('restart_tunnel', { id }),
		startTag: (tag) => call('start_tag', { tag }),
		stopTag: (tag) => call('stop_tag', { tag }),
		standbyTunnel: (id) => call('standby_tunnel', { id }),
		standbyTag: (tag) => call('standby_tag', { tag }),
		standbyAll: () => call('standby_all'),
		renameTag: (from, to) => call('rename_tag', { from, to }),
		deleteTag: (tag) => call('delete_tag', { tag }),
		startX11: (host) => call('start_x11', { host }),
		stopX11: (host) => call('stop_x11', { host }),
		launchX11: (host, name) => call('launch_x11', { host, name }),
		tunnelLogs: (id) => call('tunnel_logs', { id }),
		resolveHost: (host) => call('resolve_host', { host }),
		importSshConfig: () => call('import_ssh_config'),
		importScript: (text) => call('import_script', { text }),
		openTunnel: (id, opener) => call('open_tunnel', { id, with: opener }),
		adoptExternal: (id) => call('adopt_external', { id }),
		killExternal: (pid) => call('kill_external', { pid }),
		ignoreExternal: (pid) => call('ignore_external', { pid }),
		updateSettings: (settings, openers) =>
			call('update_settings', { settings, openers }),
		respondCredential: (promptId, answer, remember) =>
			call('respond_credential', {
				promptId,
				answer,
				remember: remember ?? null,
			}),
		forgetSecrets: () => call('forget_secrets'),
		revealConfig: () => call('reveal_config'),
		checkForUpdate: () => call('check_for_update'),
		installUpdate: (onProgress) => {
			const channel = new Channel<UpdateProgressDto>();
			channel.onmessage = onProgress;
			return call('install_update', { onProgress: channel });
		},
		openReleasePage: () => call('open_release_page'),
		openDocumentation: (page, locale) =>
			call('open_documentation', { page, locale }),
		respondHostKey: (promptId, decision) =>
			call('respond_host_key', { promptId, decision }),
		quit: (force) => call('quit_daemon', { force }),
		onEvent: (listener): Promise<Unsubscribe> =>
			listen<EventMessage>(EVENT_CHANNEL, (event) => listener(event.payload)),
		onDisconnected: (listener): Promise<Unsubscribe> =>
			listen<{ message: string }>(DISCONNECTED_CHANNEL, (event) =>
				listener(event.payload.message)
			),
	};
}
