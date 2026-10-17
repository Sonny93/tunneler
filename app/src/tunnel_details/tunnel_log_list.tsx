/* oxlint-disable jsx-a11y/no-noninteractive-tabindex -- a scrollable region must be focusable so the keyboard can scroll it */
import { useRef } from 'react';

import type { LogEntryDto } from '#/daemon/protocol';
import { LogEntryItem } from '#/tunnel_details/log_entry_item';
import { useStickToBottom } from '#/tunnel_details/use_stick_to_bottom';

type TunnelLogListProps = {
	entries: readonly LogEntryDto[];
	tunnelName: string;
};

/** Scrollable and focusable so the keyboard can scroll it; not live, state changes are announced elsewhere. */
export function TunnelLogList({
	entries,
	tunnelName,
}: Readonly<TunnelLogListProps>) {
	const containerRef = useRef<HTMLDivElement>(null);
	const { handleScroll } = useStickToBottom(containerRef, entries.length);

	return (
		<div
			ref={containerRef}
			role="log"
			aria-live="off"
			aria-label={`Journal du tunnel ${tunnelName}`}
			tabIndex={0}
			onScroll={handleScroll}
			className="max-h-72 overflow-y-auto rounded-md border border-tn-border p-2"
		>
			{entries.length === 0 ? (
				<p className="m-0 text-sm text-tn-muted">Aucun événement enregistré</p>
			) : (
				<ol className="m-0 flex list-none flex-col gap-1 p-0">
					{entries.map((entry, index) => (
						<LogEntryItem key={`${entry.at}-${index}`} entry={entry} />
					))}
				</ol>
			)}
		</div>
	);
}
