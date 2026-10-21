import { MenuGroup, MenuItem } from '@minimalstuff/ui';

import type { OpenChoice } from '#/openers/open_choices';

type OpenWithMenuItemsProps = {
	choices: readonly OpenChoice[];
	onChoose: (opener: string) => void;
};

/** "Ouvrir avec" : the other ways to open a tunnel, as a labelled group (the menu has no submenu). */
export function OpenWithMenuItems({
	choices,
	onChoose,
}: Readonly<OpenWithMenuItemsProps>) {
	if (choices.length === 0) return null;

	return (
		<MenuGroup label="Ouvrir avec">
			{choices.map((choice) => (
				<MenuItem
					key={choice.id}
					icon="i-mdi-open-in-app"
					onClick={() => onChoose(choice.id)}
				>
					{choice.name}
				</MenuItem>
			))}
		</MenuGroup>
	);
}
