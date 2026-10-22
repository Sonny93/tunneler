import { useLingui } from '@lingui/react/macro';
import { MenuGroup, MenuItem } from '@minimalstuff/ui';

import type { OpenChoice } from '#/openers/open_choices';

type OpenWithMenuItemsProps = {
	choices: readonly OpenChoice[];
	onChoose: (opener: string) => void;
};

/** "Open with": the other ways to open a tunnel, as a labelled group (the menu has no submenu). */
export function OpenWithMenuItems({
	choices,
	onChoose,
}: Readonly<OpenWithMenuItemsProps>) {
	const { t } = useLingui();

	if (choices.length === 0) return null;

	return (
		<MenuGroup label={t`Open with`}>
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
