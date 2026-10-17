import type { TunnelDefinitionDto } from '#/daemon/protocol';

const DEFAULT_TARGET_HOST = '127.0.0.1';

type SshCommandSource = Pick<
	TunnelDefinitionDto,
	'kind' | 'host' | 'listenPort' | 'listenAddress' | 'targetHost' | 'targetPort'
>;

function bracketIpv6(address: string): string {
	const isBareIpv6 = address.includes(':') && !address.startsWith('[');
	return isBareIpv6 ? `[${address}]` : address;
}

function buildListenSpec(tunnel: Readonly<SshCommandSource>): string {
	const { listenAddress, listenPort } = tunnel;
	return listenAddress
		? `${bracketIpv6(listenAddress)}:${listenPort}`
		: `${listenPort}`;
}

function buildTargetSpec(tunnel: Readonly<SshCommandSource>): string {
	const targetHost = tunnel.targetHost ?? DEFAULT_TARGET_HOST;
	return `${bracketIpv6(targetHost)}:${tunnel.targetPort ?? tunnel.listenPort}`;
}

function buildForwardOption(tunnel: Readonly<SshCommandSource>): string {
	switch (tunnel.kind) {
		case 'local':
			return `-L ${buildListenSpec(tunnel)}:${buildTargetSpec(tunnel)}`;
		case 'remote':
			return `-R ${buildListenSpec(tunnel)}:${buildTargetSpec(tunnel)}`;
		case 'dynamic':
			return `-D ${buildListenSpec(tunnel)}`;
	}
}

/**
 * The `ssh` command equivalent to a tunnel. The bind address appears only when
 * the tunnel sets one: without it, the app's default bind address applies, not
 * ssh's own default.
 */
export function buildSshCommand(tunnel: Readonly<SshCommandSource>): string {
	return `ssh -N ${buildForwardOption(tunnel)} ${tunnel.host}`;
}
