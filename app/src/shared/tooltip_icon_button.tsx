import { IconButton, Tooltip, type IconButtonProps } from '@minimalstuff/ui';

type TooltipIconButtonProps = IconButtonProps & {
	/** Shown after the name in the tooltip, e.g. `Ctrl+K`. */
	shortcutHint?: string;
};

function buildTooltipContent(label: string, shortcutHint?: string): string {
	return shortcutHint === undefined ? label : `${label} (${shortcutHint})`;
}

/**
 * An `IconButton` whose accessible name is also shown as a tooltip on hover
 * and focus, followed by its keyboard shortcut when it has one. Extra props
 * (a `Menu` trigger's wiring, roving attributes…) reach the button itself.
 */
export function TooltipIconButton({
	shortcutHint,
	...buttonProps
}: Readonly<TooltipIconButtonProps>) {
	const content = buildTooltipContent(buttonProps['aria-label'], shortcutHint);

	return (
		<Tooltip content={content}>
			<IconButton {...buttonProps} />
		</Tooltip>
	);
}
