import { cn } from '#/shared/cn';

type TemporaryBadgeProps = {
	className?: string;
};

/** Icon and word, so the temporary nature is never carried by color alone. */
export const TemporaryBadge = ({
	className,
}: Readonly<TemporaryBadgeProps>) => (
	<span
		className={cn(
			'inline-flex shrink-0 items-center gap-1.5 rounded-md bg-tn-neutral-subtle px-2 py-0.5 text-xs font-medium text-tn-neutral',
			className
		)}
	>
		<span aria-hidden="true" className="i-mdi-timer-sand size-4 shrink-0" />
		Temporaire
	</span>
);
