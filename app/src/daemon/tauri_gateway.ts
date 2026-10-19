import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { EventMessage } from '#/daemon/protocol';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { DaemonGateway, Unsubscribe } from '#/daemon/daemon_gateway';

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
		deleteTunnel: (id) => call('delete_tunnel', { id }),
		startTunnel: (id) => call('start_tunnel', { id }),
		stopTunnel: (id) => call('stop_tunnel', { id }),
		restartTunnel: (id) => call('restart_tunnel', { id }),
		startTag: (tag) => call('start_tag', { tag }),
		stopTag: (tag) => call('stop_tag', { tag }),
		renameTag: (from, to) => call('rename_tag', { from, to }),
		deleteTag: (tag) => call('delete_tag', { tag }),
		startX11: (host) => call('start_x11', { host }),
		stopX11: (host) => call('stop_x11', { host }),
		launchX11: (host, name) => call('launch_x11', { host, name }),
		tunnelLogs: (id) => call('tunnel_logs', { id }),
		resolveHost: (host) => call('resolve_host', { host }),
		importSshConfig: () => call('import_ssh_config'),
		importScript: (text) => call('import_script', { text }),
		openInBrowser: (host, port) => call('open_in_browser', { host, port }),
		adoptExternal: (id) => call('adopt_external', { id }),
		killExternal: (pid) => call('kill_external', { pid }),
		ignoreExternal: (pid) => call('ignore_external', { pid }),
		updateSettings: (settings) => call('update_settings', { settings }),
		respondCredential: (promptId, answer, remember) =>
			call('respond_credential', {
				promptId,
				answer,
				remember: remember ?? null,
			}),
		forgetSecrets: () => call('forget_secrets'),
		revealConfig: () => call('reveal_config'),
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
