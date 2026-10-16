import { cn } from '#/shared/cn';
import type { ActiveCount } from '#/tunnels/active_count';
import { formatActiveCount } from '#/tunnels/format_active_count';
import { describeActiveCount } from '#/tunnels/describe_active_count';

type FilterButtonProps = {
	label: string;
	count: ActiveCount;
	isActive: boolean;
	onSelect: () => void;
};

/**
 * `aria-current` (not `aria-pressed`): the buttons are the entries of a
 * navigation, exactly one is current, and none is a toggle. The active entry
 * is also bold with a left bar and a check icon, so color is not the only cue.
 */
export const FilterButton = ({
	label,
	count,
	isActive,
	onSelect,
}: Readonly<FilterButtonProps>) => (
	<button
		type="button"
		aria-current={isActive ? 'true' : undefined}
		onClick={onSelect}
		className={cn(
			'flex min-h-8 min-w-0 flex-1 items-center gap-2 rounded-md border-l-4 px-2 py-1 text-left text-sm text-tn-text hover:bg-tn-surface-raised',
			isActive
				? 'border-tn-accent bg-tn-surface-raised font-semibold'
				: 'border-transparent'
		)}
	>
		<span
			aria-hidden="true"
			className={cn(
				'size-4 shrink-0',
				isActive && 'i-mdi-check text-tn-accent'
			)}
		/>
		<span className="min-w-0 flex-1 break-words">{label}</span>
		<span
			aria-hidden="true"
			className="shrink-0 text-xs font-normal text-tn-muted"
		>
			{formatActiveCount(count)}
		</span>
		<span className="sr-only">{describeActiveCount(count)}</span>
	</button>
);
