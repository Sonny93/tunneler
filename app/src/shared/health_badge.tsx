import { cn } from '#/shared/cn';
import { BadgeDetail } from '#/shared/badge_detail';
import type { TargetHealthDto } from '#/daemon/protocol';
import {
	getTargetHealthError,
	getTargetHealthLabel,
} from '#/tunnels/target_health_label';

type HealthAppearance = { icon: string; toneClassName: string };

const HEALTH_APPEARANCES: Readonly<
	Record<Exclude<TargetHealthDto['kind'], 'unknown'>, HealthAppearance>
> = {
	reachable: {
		icon: 'i-mdi-check-network-outline',
		toneClassName: 'text-tn-success bg-tn-success-subtle',
	},
	unreachable: {
		icon: 'i-mdi-network-off-outline',
		toneClassName: 'text-tn-danger bg-tn-danger-subtle',
	},
};

type TargetHealthBadgeProps = {
	health: TargetHealthDto;
	isDetailTruncated?: boolean;
	className?: string;
};

export function TargetHealthBadge({
	health,
	isDetailTruncated = false,
	className,
}: Readonly<TargetHealthBadgeProps>) {
	if (health.kind === 'unknown') return null;
	const { icon, toneClassName } = HEALTH_APPEARANCES[health.kind];
	const error = getTargetHealthError(health);

	return (
		<span
			className={cn(
				'tn-status-badge inline-flex max-w-full min-w-0 items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium',
				toneClassName,
				className
			)}
		>
			<span aria-hidden="true" className={cn(icon, 'size-4 shrink-0')} />
			<span className="shrink-0">{getTargetHealthLabel(health)}</span>
			{error && (
				<BadgeDetail
					text={error}
					isTruncated={isDetailTruncated}
					className="font-normal opacity-90"
				/>
			)}
		</span>
	);
}
