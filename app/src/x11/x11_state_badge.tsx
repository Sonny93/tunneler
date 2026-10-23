import { useLingui } from '@lingui/react/macro';

import type { X11StateDto } from '#/daemon/protocol';
import { cn } from '#/shared/cn';
import { getX11StateLabel } from '#/x11/x11_state_label';

type StateAppearance = { icon: string; toneClassName: string };

const STATE_APPEARANCES: Readonly<
	Record<X11StateDto['kind'], StateAppearance>
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

type X11StateBadgeProps = {
	state: X11StateDto;
};

export const X11StateBadge = ({ state }: Readonly<X11StateBadgeProps>) => {
	const { i18n } = useLingui();
	const { icon, toneClassName } = STATE_APPEARANCES[state.kind];

	return (
		<span
			className={cn(
				'inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium',
				toneClassName
			)}
		>
			<span aria-hidden="true" className={cn(icon, 'size-4 shrink-0')} />
			<span>{getX11StateLabel(state, i18n)}</span>
		</span>
	);
};
