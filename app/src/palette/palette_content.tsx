import { useLingui } from '@lingui/react/macro';
/* oxlint-disable jsx-a11y/prefer-tag-over-role, jsx-a11y/no-noninteractive-element-to-interactive-role -- ARIA combobox pattern: a custom palette cannot use a native <select> or <datalist> */
import { Input } from '@minimalstuff/ui';
import { useId, useState, type ChangeEvent, type KeyboardEvent } from 'react';

import { buildPaletteCommands } from '#/palette/build_palette_commands';
import { describeCommandCount } from '#/palette/describe_command_count';
import { filterPaletteCommands } from '#/palette/filter_palette_commands';
import {
	moveActiveIndex,
	type ActiveIndexMove,
} from '#/palette/move_active_index';
import type { PaletteAction } from '#/palette/palette_command';
import { PaletteOption } from '#/palette/palette_option';
import { useDaemonStore } from '#/state/daemon_store';

const MOVE_BY_KEY: Readonly<Record<string, ActiveIndexMove>> = {
	ArrowDown: 'next',
	ArrowUp: 'previous',
	Home: 'first',
	End: 'last',
};

type PaletteContentProps = {
	onRun: (action: PaletteAction) => void;
};

function getOptionId(listboxId: string, index: number): string {
	return `${listboxId}-option-${index}`;
}

/** The ARIA combobox pattern: the input keeps the focus, the active option is referenced by id. */
export function PaletteContent({ onRun }: Readonly<PaletteContentProps>) {
	const { t, i18n } = useLingui();
	const listboxId = useId();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const hosts = useDaemonStore((state) => state.hosts);
	const openers = useDaemonStore((state) => state.openers);
	const [query, setQuery] = useState('');
	const [activeIndex, setActiveIndex] = useState(0);
	const commands = buildPaletteCommands({ tunnels, hosts, openers }, i18n);
	const results = filterPaletteCommands(commands, query);
	const activeCommand = results[activeIndex];

	const handleQueryChange = (event: ChangeEvent<HTMLInputElement>) => {
		setQuery(event.target.value);
		setActiveIndex(0);
	};

	const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
		if (event.key === 'Enter') {
			event.preventDefault();
			if (activeCommand) onRun(activeCommand.action);
			return;
		}
		const move = MOVE_BY_KEY[event.key];
		if (move === undefined) return;
		event.preventDefault();
		setActiveIndex(
			(current) => moveActiveIndex(current, results.length, move) ?? 0
		);
	};

	return (
		<div className="flex flex-col gap-2">
			<Input
				role="combobox"
				aria-label={t`Search a command`}
				aria-expanded={results.length > 0}
				aria-controls={listboxId}
				aria-autocomplete="list"
				aria-activedescendant={
					activeCommand ? getOptionId(listboxId, activeIndex) : undefined
				}
				placeholder={t`Search a command`}
				value={query}
				onChange={handleQueryChange}
				onKeyDown={handleKeyDown}
				autoComplete="off"
				spellCheck={false}
			/>
			<output className="block text-xs text-tn-muted">
				{describeCommandCount(results.length, i18n)}
			</output>
			<ul
				id={listboxId}
				role="listbox"
				aria-label={t`Commands`}
				className="m-0 flex max-h-80 list-none flex-col overflow-y-auto p-0"
			>
				{results.map((command, index) => (
					<PaletteOption
						key={command.id}
						id={getOptionId(listboxId, index)}
						command={command}
						isActive={index === activeIndex}
						onRun={onRun}
					/>
				))}
			</ul>
		</div>
	);
}
