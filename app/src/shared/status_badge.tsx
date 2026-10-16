import { cn } from '#/shared/cn';
import type { TunnelStateDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

type StateAppearance = { icon: string; toneClassName: string };

const STATE_APPEARANCES: Readonly<
	Record<TunnelStateDto['kind'], StateAppearance>
> = {
	stopped: {
		icon: 'i-mdi-stop-circle-outline',
		toneClassName: 'text-tn-neutral bg-tn-neutral-subtle',
	},
	starting: {
		icon: 'i-mdi-progress-clock',
		toneClassName: 'text-tn-info bg-tn-info-subtle',
	},
	active: {
		icon: 'i-mdi-check-circle',
		toneClassName: 'text-tn-success bg-tn-success-subtle',
	},
	reconnecting: {
		icon: 'i-mdi-sync',
		toneClassName: 'text-tn-warning bg-tn-warning-subtle',
	},
	failed: {
		icon: 'i-mdi-alert-octagon',
		toneClassName: 'text-tn-danger bg-tn-danger-subtle',
	},
};

type TunnelStatusBadgeProps = {
	state: TunnelStateDto;
	className?: string;
};

export const TunnelStatusBadge = ({
	state,
	className,
}: Readonly<TunnelStatusBadgeProps>) => {
	const { icon, toneClassName } = STATE_APPEARANCES[state.kind];

	return (
		<span
			className={cn(
				'tn-status-badge inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium',
				toneClassName,
				className
			)}
		>
			<span aria-hidden="true" className={cn(icon, 'size-4 shrink-0')} />
			<span>{getTunnelStateLabel(state)}</span>
			{state.kind === 'failed' && (
				<span className="font-normal">
					{getFailureReasonLabel(state.reason)}
				</span>
			)}
		</span>
	);
};
