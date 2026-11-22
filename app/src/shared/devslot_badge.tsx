import { useLingui } from '@lingui/react/macro';

import { cn } from '#/shared/cn';

const DEVSLOT_FALLBACK_LABEL = 'devslot';

type DevslotBadgeProps = {
	projectTag?: string;
	className?: string;
};

/** The project of a temporary tunnel started by devslot discovery, as icon and text. */
export function DevslotBadge({
	projectTag,
	className,
}: Readonly<DevslotBadgeProps>) {
	const { t } = useLingui();

	return (
		<span
			className={cn(
				'inline-flex max-w-48 shrink-0 items-center gap-1.5 rounded-md bg-tn-neutral-subtle px-2 py-0.5 text-xs font-medium text-tn-neutral',
				className
			)}
		>
			<span
				aria-hidden="true"
				className="i-mdi-source-branch size-4 shrink-0"
			/>
			<span className="truncate">
				<span className="sr-only">{t`devslot project: `}</span>
				{projectTag ?? DEVSLOT_FALLBACK_LABEL}
			</span>
		</span>
	);
}
