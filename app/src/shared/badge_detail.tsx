import { Tooltip } from '@minimalstuff/ui';

import { cn } from '#/shared/cn';

type BadgeDetailProps = {
	text: string;
	isTruncated: boolean;
	className?: string;
};

/**
 * The reason shown after a badge label. When truncated it stays on one line
 * with an ellipsis and the full text moves to a hover tooltip: the DOM text
 * stays whole, and the row exposes it to screen readers through the switch
 * description.
 */
export function BadgeDetail({
	text,
	isTruncated,
	className,
}: Readonly<BadgeDetailProps>) {
	if (!isTruncated) return <span className={className}>{text}</span>;

	return (
		<Tooltip content={text} describeChild={false} className="min-w-0">
			<span className={cn('block truncate', className)}>{text}</span>
		</Tooltip>
	);
}
