/* oxlint-disable jsx-a11y/click-events-have-key-events, jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-element-to-interactive-role, jsx-a11y/prefer-tag-over-role -- ARIA combobox: the input keeps the focus and handles the keys, an option only needs the mouse */
import { useEffect, useRef } from 'react';

import { cn } from '#/shared/cn';
import type { PaletteAction, PaletteCommand } from '#/palette/palette_command';

type PaletteOptionProps = {
	id: string;
	command: PaletteCommand;
	isActive: boolean;
	onRun: (action: PaletteAction) => void;
};

export function PaletteOption({
	id,
	command,
	isActive,
	onRun,
}: Readonly<PaletteOptionProps>) {
	const optionRef = useRef<HTMLLIElement>(null);

	useEffect(() => {
		if (isActive) optionRef.current?.scrollIntoView?.({ block: 'nearest' });
	}, [isActive]);

	const handleClick = () => onRun(command.action);
	const keepInputFocus = (event: { preventDefault: () => void }) =>
		event.preventDefault();

	return (
		<li
			ref={optionRef}
			id={id}
			role="option"
			aria-selected={isActive}
			onClick={handleClick}
			onMouseDown={keepInputFocus}
			className={cn(
				'flex min-h-8 cursor-pointer items-center justify-between gap-3 rounded-md px-3 py-1.5 text-sm text-tn-text',
				isActive &&
					'bg-tn-surface-raised outline outline-2 outline-tn-focus-ring'
			)}
		>
			<span className="min-w-0 break-words">{command.label}</span>
			<span className="shrink-0 text-xs text-tn-muted">{command.category}</span>
		</li>
	);
}
