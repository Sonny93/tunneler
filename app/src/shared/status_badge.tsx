import { cn } from '#/shared/cn';
import { BadgeDetail } from '#/shared/badge_detail';
import type { TunnelStateDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';

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
	external: {
		icon: 'i-mdi-link-variant',
		toneClassName: 'text-tn-info bg-tn-info-subtle',
	},
	conflict: {
		icon: 'i-mdi-alert',
		toneClassName: 'text-tn-warning bg-tn-warning-subtle',
	},
};

type TunnelStatusBadgeProps = {
	state: TunnelStateDto;
	isDetailTruncated?: boolean;
	className?: string;
};

export const TunnelStatusBadge = ({
	state,
	isDetailTruncated = false,
	className,
}: Readonly<TunnelStatusBadgeProps>) => {
	const { icon, toneClassName } = STATE_APPEARANCES[state.kind];
	const detail = getTunnelStateDetail(state);

	return (
		<span
			className={cn(
				'tn-status-badge inline-flex max-w-full min-w-0 items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium',
				toneClassName,
				className
			)}
		>
			<span aria-hidden="true" className={cn(icon, 'size-4 shrink-0')} />
			<span className="shrink-0">{getTunnelStateLabel(state)}</span>
			{detail !== null && (
				<BadgeDetail
					text={detail}
					isTruncated={isDetailTruncated}
					className="font-normal"
				/>
			)}
		</span>
	);
};
