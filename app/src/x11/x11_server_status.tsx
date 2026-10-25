import { Trans, useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import { cn } from '#/shared/cn';
import type { X11ServerStatus as X11ServerStatusState } from '#/x11/x11_server_status_state';

type StatusAppearance = { icon: string; toneClassName: string };

const STATUS_APPEARANCES: Readonly<
	Record<X11ServerStatusState['kind'], StatusAppearance>
> = {
	checking: { icon: 'i-mdi-progress-clock', toneClassName: 'text-tn-info' },
	reachable: { icon: 'i-mdi-check-circle', toneClassName: 'text-tn-success' },
	unreachable: { icon: 'i-mdi-alert', toneClassName: 'text-tn-warning' },
	failed: { icon: 'i-mdi-alert-octagon', toneClassName: 'text-tn-danger' },
};

type X11ServerStatusProps = {
	status: X11ServerStatusState;
	isStarting: boolean;
	onStart: (server: string) => void;
};

function StatusText({ status }: Readonly<{ status: X11ServerStatusState }>) {
	switch (status.kind) {
		case 'checking':
			return <Trans>Checking the X server…</Trans>;
		case 'reachable':
			return <Trans>An X server answers on {status.display}.</Trans>;
		case 'failed':
			return <Trans>Couldn't check the X server: {status.reason}.</Trans>;
		case 'unreachable':
			return status.server === null ? (
				<Trans>
					No X server answers on {status.display}. Start your X server, or on
					Windows install one such as VcXsrv (winget install marha.VcXsrv).
				</Trans>
			) : (
				<Trans>No X server answers on {status.display}.</Trans>
			);
	}
}

/**
 * The state of the local X server. The text sits in a polite status region
 * that only changes when the state changes, so a poll that finds the same
 * thing announces nothing.
 */
export function X11ServerStatus({
	status,
	isStarting,
	onStart,
}: Readonly<X11ServerStatusProps>) {
	const { t } = useLingui();
	const { icon, toneClassName } = STATUS_APPEARANCES[status.kind];
	const server = status.kind === 'unreachable' ? status.server : null;

	return (
		<div className="flex flex-col gap-2">
			<output className="m-0 flex items-start gap-1.5 text-sm">
				<span
					aria-hidden="true"
					className={cn(icon, 'mt-0.5 size-4 shrink-0', toneClassName)}
				/>
				<span>
					<StatusText status={status} />
				</span>
			</output>
			{server !== null && (
				<div>
					<Button
						startIcon="i-mdi-play"
						loading={isStarting}
						onClick={() => onStart(server)}
					>
						{t`Start ${server}`}
					</Button>
				</div>
			)}
		</div>
	);
}
