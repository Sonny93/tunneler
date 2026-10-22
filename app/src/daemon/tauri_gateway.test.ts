import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createTauriGateway } from '#/daemon/tauri_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	Channel: class {
		onmessage: (message: unknown) => void = () => undefined;
	},
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const listenMock = vi.mocked(listen);

describe('createTauriGateway', () => {
	beforeEach(() => {
		invokeMock.mockReset();
		listenMock.mockReset();
		invokeMock.mockResolvedValue(undefined);
	});

	it('should invoke connect_daemon without arguments', async () => {
		await createTauriGateway().connect();

		expect(invokeMock).toHaveBeenCalledWith('connect_daemon', undefined);
	});

	it('should return the snapshot given by the native side', async () => {
		invokeMock.mockResolvedValue({ tunnels: [] });

		expect(await createTauriGateway().snapshot()).toEqual({ tunnels: [] });
	});

	it.each([
		['listHosts', [], 'list_hosts', undefined],
		['createTunnel', [{ name: 'a' }], 'create_tunnel', { spec: { name: 'a' } }],
		[
			'updateTunnel',
			[{ id: '1' }],
			'update_tunnel',
			{ definition: { id: '1' } },
		],
		[
			'startTemporaryTunnel',
			[{ host: 'nova' }],
			'start_temporary_tunnel',
			{ spec: { host: 'nova' } },
		],
		['persistTunnel', ['1'], 'persist_tunnel', { id: '1' }],
		['checkTarget', ['1'], 'check_target', { id: '1' }],
		[
			'previewSound',
			['chime', 'woke'],
			'preview_sound',
			{ sound: 'chime', event: 'woke' },
		],
		['deleteTunnel', ['1'], 'delete_tunnel', { id: '1' }],
		['startTunnel', ['1'], 'start_tunnel', { id: '1' }],
		['stopTunnel', ['1'], 'stop_tunnel', { id: '1' }],
		['restartTunnel', ['1'], 'restart_tunnel', { id: '1' }],
		['startTag', ['dev'], 'start_tag', { tag: 'dev' }],
		['stopTag', ['dev'], 'stop_tag', { tag: 'dev' }],
		['standbyTunnel', ['1'], 'standby_tunnel', { id: '1' }],
		['standbyTag', ['dev'], 'standby_tag', { tag: 'dev' }],
		['standbyAll', [], 'standby_all', undefined],
		[
			'renameTag',
			['dev', 'staging'],
			'rename_tag',
			{ from: 'dev', to: 'staging' },
		],
		['deleteTag', ['dev'], 'delete_tag', { tag: 'dev' }],
		['startX11', ['nova'], 'start_x11', { host: 'nova' }],
		['stopX11', ['nova'], 'stop_x11', { host: 'nova' }],
		[
			'launchX11',
			['nova', 'VS Code'],
			'launch_x11',
			{ host: 'nova', name: 'VS Code' },
		],
		['tunnelLogs', ['1'], 'tunnel_logs', { id: '1' }],
		['resolveHost', ['nova'], 'resolve_host', { host: 'nova' }],
		['importSshConfig', [], 'import_ssh_config', undefined],
		['importScript', ['3333'], 'import_script', { text: '3333' }],
		['openTunnel', ['1'], 'open_tunnel', { id: '1', with: undefined }],
		['openTunnel', ['1', 'viewer'], 'open_tunnel', { id: '1', with: 'viewer' }],
		[
			'updateOpeners',
			[[{ id: 'a' }]],
			'update_openers',
			{ openers: [{ id: 'a' }] },
		],
		['updateSettings', [{ a: 1 }], 'update_settings', { settings: { a: 1 } }],
		[
			'respondCredential',
			['p1', 'secret'],
			'respond_credential',
			{ promptId: 'p1', answer: 'secret', remember: null },
		],
		[
			'respondCredential',
			['p1', 'secret', 'keyring'],
			'respond_credential',
			{ promptId: 'p1', answer: 'secret', remember: 'keyring' },
		],
		['forgetSecrets', [], 'forget_secrets', undefined],
		['revealConfig', [], 'reveal_config', undefined],
		['checkForUpdate', [], 'check_for_update', undefined],
		['openReleasePage', [], 'open_release_page', undefined],
		[
			'respondHostKey',
			['p1', 'reject'],
			'respond_host_key',
			{ promptId: 'p1', decision: 'reject' },
		],
		['quit', [true], 'quit_daemon', { force: true }],
	] as const)(
		'should map %s to the %s command with the expected arguments',
		async (method, args, command, expectedArguments) => {
			const gateway = createTauriGateway() as unknown as Record<
				string,
				(...methodArguments: readonly unknown[]) => Promise<unknown>
			>;

			await gateway[method]?.(...args);

			expect(invokeMock).toHaveBeenCalledWith(command, expectedArguments);
		}
	);

	it('should pass a channel relaying the progress to the install_update command', async () => {
		const onProgress = vi.fn();

		await createTauriGateway().installUpdate(onProgress);

		const [command, args] = invokeMock.mock.calls[0] ?? [];
		const channel = (
			args as { onProgress: { onmessage: (m: unknown) => void } }
		).onProgress;
		channel.onmessage({ kind: 'installing' });
		expect(command).toBe('install_update');
		expect(onProgress).toHaveBeenCalledWith({ kind: 'installing' });
	});

	it('should convert a native rejection into a DaemonCommandError', async () => {
		invokeMock.mockRejectedValue({
			code: 'tunnels_running',
			message: 'running',
			data: 2,
		});

		const rejection = await createTauriGateway()
			.quit(false)
			.catch((error: unknown) => error);

		expect(rejection).toMatchObject({
			code: 'tunnels_running',
			data: 2,
		});
		expect(rejection).toBeInstanceOf(DaemonCommandError);
	});

	it('should forward daemon events payloads to the listener', async () => {
		const unlisten = vi.fn();
		listenMock.mockResolvedValue(unlisten);
		const listener = vi.fn();

		const unsubscribe = await createTauriGateway().onEvent(listener);
		const [channel, handler] = listenMock.mock.calls[0] ?? [];
		const payload = { event: 'quit.requested', data: {} };
		handler?.({ event: 'x', id: 1, payload });

		expect(channel).toBe('daemon-event');
		expect(listener).toHaveBeenCalledWith(payload);
		expect(unsubscribe).toBe(unlisten);
	});

	it('should forward the disconnection message to the listener', async () => {
		listenMock.mockResolvedValue(vi.fn());
		const listener = vi.fn();

		await createTauriGateway().onDisconnected(listener);
		const [channel, handler] = listenMock.mock.calls[0] ?? [];
		handler?.({ event: 'x', id: 1, payload: { message: 'lost' } });

		expect(channel).toBe('daemon-disconnected');
		expect(listener).toHaveBeenCalledWith('lost');
	});
});
