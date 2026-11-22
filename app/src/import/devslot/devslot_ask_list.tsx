import { useLingui } from '@lingui/react/macro';
import { useId, useRef, useState } from 'react';

import type { DevslotStaleTunnelDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { DevslotAskItem } from '#/import/devslot/devslot_ask_item';
import { describeSkippedStale } from '#/import/devslot/devslot_texts';
import { toResyncTunnel } from '#/import/devslot/to_resync_tunnel';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type DevslotAskListProps = {
	host: string;
	entries: readonly DevslotStaleTunnelDto[];
	onStatus: (message: string) => void;
	onRemoved: (ids: readonly string[]) => void;
	onFailure: (rejection: unknown, title: string) => void;
};

/** Saved tunnels the discovery could not decide about: the user deletes or keeps each one. */
export function DevslotAskList({
	host,
	entries,
	onStatus,
	onRemoved,
	onFailure,
}: Readonly<DevslotAskListProps>) {
	const { t, i18n } = useLingui();
	const gateway = useDaemonGateway();
	const headingId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const actionRefs = useRef(new Map<string, HTMLButtonElement>());
	const [pending, setPending] = useState(entries);
	const [deletingId, setDeletingId] = useState<string | null>(null);

	const hide = (id: string) =>
		setPending((current) => current.filter(({ tunnel }) => tunnel.id !== id));

	const moveFocusAfter = (id: string) => {
		const position = pending.findIndex(({ tunnel }) => tunnel.id === id);
		const next = pending[position + 1]?.tunnel.id;
		const target = next === undefined ? null : actionRefs.current.get(next);
		(target ?? headingRef.current)?.focus();
	};

	const handleKeep = (entry: Readonly<DevslotStaleTunnelDto>) => {
		const route = getTunnelAccessibleEndpointLabel(entry.tunnel, i18n);
		moveFocusAfter(entry.tunnel.id);
		hide(entry.tunnel.id);
		onStatus(t`Kept ${route}`);
	};

	const deleteEntry = async (entry: Readonly<DevslotStaleTunnelDto>) => {
		if (deletingId !== null) return;
		const route = getTunnelAccessibleEndpointLabel(entry.tunnel, i18n);
		setDeletingId(entry.tunnel.id);
		try {
			const outcome = await gateway.resyncDevslot(host, [
				toResyncTunnel(entry),
			]);
			const isSkipped = outcome.skipped.includes(entry.tunnel.id);
			if (!isSkipped) onRemoved(outcome.removed);
			onStatus(isSkipped ? describeSkippedStale(1, i18n) : t`Deleted ${route}`);
		} catch (rejection) {
			setDeletingId(null);
			onFailure(rejection, t`Could not delete ${route}:`);
			return;
		}
		setDeletingId(null);
		moveFocusAfter(entry.tunnel.id);
		hide(entry.tunnel.id);
	};

	const handleDelete = (entry: Readonly<DevslotStaleTunnelDto>) =>
		void deleteEntry(entry);

	const registerAction = (id: string, element: HTMLButtonElement | null) => {
		if (element === null) actionRefs.current.delete(id);
		else actionRefs.current.set(id, element);
	};

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h4
				id={headingId}
				ref={headingRef}
				tabIndex={-1}
				className="m-0 text-sm font-semibold"
			>
				{t`Saved tunnels to review`}
			</h4>
			<p className="m-0 text-sm text-tn-muted">
				{t`devslot no longer lists these saved tunnels as they are. Delete removes one; Keep leaves it as it is.`}
			</p>
			{pending.length === 0 && (
				<p className="m-0 text-sm">{t`Nothing left to review.`}</p>
			)}
			<ul className="m-0 flex list-none flex-col gap-2 p-0">
				{pending.map((entry) => (
					<DevslotAskItem
						key={entry.tunnel.id}
						entry={entry}
						isBusy={deletingId === entry.tunnel.id}
						registerAction={registerAction}
						onDelete={handleDelete}
						onKeep={handleKeep}
					/>
				))}
			</ul>
		</section>
	);
}
