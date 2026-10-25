import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useCallback, useEffect, useRef, useState } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { X11ServerStatus } from '#/x11/x11_server_status_state';

export const X11_SERVER_POLL_INTERVAL_MS = 5000;

type X11ServerStatusControls = {
	status: X11ServerStatus;
	isStarting: boolean;
	startServer: (server: string) => Promise<void>;
};

/**
 * Probes the local X server of `host` while `isActive`: immediately, then
 * every `X11_SERVER_POLL_INTERVAL_MS`, never two probes at the same time.
 */
export function useX11ServerStatus(
	host: string,
	isActive: boolean
): X11ServerStatusControls {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const runDaemonAction = useDaemonAction();
	const [status, setStatus] = useState<X11ServerStatus>({ kind: 'checking' });
	const [isStarting, setIsStarting] = useState(false);
	const inFlightProbe = useRef<Promise<void> | null>(null);
	const isPolling = useRef(false);

	const probeOnce = useCallback(async () => {
		let nextStatus: X11ServerStatus;
		try {
			const probe = await gateway.probeX11(host);
			nextStatus = probe.isReachable
				? { kind: 'reachable', display: probe.display }
				: { kind: 'unreachable', display: probe.display, server: probe.server };
		} catch (rejection) {
			nextStatus = {
				kind: 'failed',
				reason: describeDaemonError(toDaemonCommandError(rejection), i18n),
			};
		}
		if (!isPolling.current) return;
		setStatus((previous) =>
			JSON.stringify(previous) === JSON.stringify(nextStatus)
				? previous
				: nextStatus
		);
	}, [gateway, host, i18n]);

	const probeUnlessBusy = useCallback(async () => {
		if (inFlightProbe.current !== null) return;
		inFlightProbe.current = probeOnce();
		await inFlightProbe.current;
		inFlightProbe.current = null;
	}, [probeOnce]);

	useEffect(() => {
		if (!isActive) return;
		isPolling.current = true;
		void probeUnlessBusy();
		const timer = setInterval(
			() => void probeUnlessBusy(),
			X11_SERVER_POLL_INTERVAL_MS
		);
		return () => {
			isPolling.current = false;
			clearInterval(timer);
		};
	}, [isActive, probeUnlessBusy]);

	const startServer = useCallback(
		async (server: string) => {
			setIsStarting(true);
			await runDaemonAction(
				(daemonGateway) => daemonGateway.startX11Server(host),
				i18n._(msg`Couldn't start ${server}`)
			);
			await inFlightProbe.current;
			await probeUnlessBusy();
			if (isPolling.current) setIsStarting(false);
		},
		[host, i18n, probeUnlessBusy, runDaemonAction]
	);

	return { status, isStarting, startServer };
}
