import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useState } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type {
	DevslotDiscoveryDto,
	DevslotStaleTunnelDto,
} from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import {
	describeDiscovering,
	describeRemovedStale,
	describeSkippedStale,
} from '#/import/devslot/devslot_texts';
import { toResyncTunnel } from '#/import/devslot/to_resync_tunnel';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const NOT_FOUND_CODE = 'devslot_not_found';
const DISCOVERY_FAILED_TITLE = msg`Discovery failed:`;
const CLEANUP_FAILED_TITLE = msg`Could not remove the stale tunnels:`;

export type DevslotAnalysis = { result: DevslotDiscoveryDto; revision: number };

type UseDevslotDiscoveryReturn = {
	/** `revision` changes with every successful discovery, to reset what depends on the result. */
	discovery: DevslotAnalysis | null;
	isDiscovering: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	summaryTitle: string;
	/** The host whose `ds` was not found, to show how to configure its path. */
	notFoundHost: string | null;
	statusMessage: string;
	/** Tunnels the daemon removed since the discovery. */
	removedIds: ReadonlySet<string>;
	reportStatus: (message: string) => void;
	reportRemoved: (ids: readonly string[]) => void;
	reportFailureWithTitle: (rejection: unknown, title: string) => void;
	discover: (host: string) => void;
};

export function useDevslotDiscovery(): UseDevslotDiscoveryReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const [discovery, setDiscovery] = useState<DevslotAnalysis | null>(null);
	const [isDiscovering, setIsDiscovering] = useState(false);
	const [notFoundHost, setNotFoundHost] = useState<string | null>(null);
	const [statusMessage, setStatusMessage] = useState('');
	const [summaryTitle, setSummaryTitle] = useState('');
	const [removedIds, setRemovedIds] = useState<ReadonlySet<string>>(new Set());
	const summary = useFormErrorSummary();

	const reportRemoved = (ids: readonly string[]) =>
		setRemovedIds((current) => new Set([...current, ...ids]));

	const reportFailureWithTitle = (rejection: unknown, title: string) => {
		setSummaryTitle(title);
		summary.reportFailure(rejection);
	};

	const removeStale = async (
		host: string,
		staleTunnels: readonly DevslotStaleTunnelDto[]
	) => {
		if (staleTunnels.length === 0) return;
		try {
			const outcome = await gateway.resyncDevslot(
				host,
				staleTunnels.map(toResyncTunnel)
			);
			reportRemoved(outcome.removed);
			const routes = staleTunnels
				.filter(({ tunnel }) => outcome.removed.includes(tunnel.id))
				.map(({ tunnel }) => getTunnelEndpointLabel(tunnel));
			const parts = [
				routes.length > 0 ? describeRemovedStale(routes, i18n) : '',
				outcome.skipped.length > 0
					? describeSkippedStale(outcome.skipped.length, i18n)
					: '',
			];
			setStatusMessage(parts.filter((part) => part !== '').join(' '));
		} catch (rejection) {
			reportFailureWithTitle(rejection, i18n._(CLEANUP_FAILED_TITLE));
		}
	};

	const run = async (host: string) => {
		setIsDiscovering(true);
		summary.clearFailure();
		setNotFoundHost(null);
		setRemovedIds(new Set());
		setStatusMessage(describeDiscovering(host, i18n));
		try {
			const result = await gateway.discoverDevslot(host);
			setStatusMessage('');
			setDiscovery((previous) => ({
				result,
				revision: (previous?.revision ?? 0) + 1,
			}));
			await removeStale(
				host,
				result.stale.filter(({ action }) => action === 'remove')
			);
		} catch (rejection) {
			setStatusMessage('');
			setDiscovery(null);
			if (toDaemonCommandError(rejection).code === NOT_FOUND_CODE) {
				setNotFoundHost(host);
			}
			reportFailureWithTitle(rejection, i18n._(DISCOVERY_FAILED_TITLE));
		} finally {
			setIsDiscovering(false);
		}
	};

	const discover = (host: string) => {
		if (isDiscovering) return;
		void run(host);
	};

	return {
		discovery,
		isDiscovering,
		summary,
		summaryTitle,
		notFoundHost,
		statusMessage,
		removedIds,
		reportStatus: setStatusMessage,
		reportRemoved,
		reportFailureWithTitle,
		discover,
	};
}
