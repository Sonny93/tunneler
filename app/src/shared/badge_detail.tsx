import { cn } from '#/shared/cn';

type BadgeDetailProps = {
	text: string;
	isTruncated: boolean;
	className?: string;
};

/**
 * The reason shown after a badge label. When truncated it stays on one line
 * with an ellipsis: the DOM text stays whole, the row exposes it to screen
 * readers through the switch description and the Détails drawer shows it in full.
 */
export function BadgeDetail({
	text,
	isTruncated,
	className,
}: Readonly<BadgeDetailProps>) {
	return (
		<span className={cn(isTruncated && 'min-w-0 truncate', className)}>
			{text}
		</span>
	);
}
