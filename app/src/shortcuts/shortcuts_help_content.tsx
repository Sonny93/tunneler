import { Kbd } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import {
	getGlobalShortcuts,
	getListShortcuts,
	type ShortcutDefinition,
} from '#/shortcuts/shortcut_definitions';

type ShortcutGroupProps = {
	title: string;
	shortcuts: readonly ShortcutDefinition[];
};

const ShortcutGroup = ({ title, shortcuts }: Readonly<ShortcutGroupProps>) => (
	<section className="mb-4">
		<h3 className="mb-2 text-sm font-semibold">{title}</h3>
		<dl className="flex flex-col gap-2">
			{shortcuts.map((shortcut) => (
				<div
					key={shortcut.description}
					className="flex items-center justify-between gap-4"
				>
					<dt className="flex gap-1">
						{shortcut.keys.map((key) => (
							<Kbd key={key}>{key}</Kbd>
						))}
					</dt>
					<dd>{shortcut.description}</dd>
				</div>
			))}
		</dl>
	</section>
);

export function ShortcutsHelpContent() {
	const { t, i18n } = useLingui();

	return (
		<>
			<ShortcutGroup
				title={t`Everywhere`}
				shortcuts={getGlobalShortcuts(i18n)}
			/>
			<ShortcutGroup
				title={t`Tunnel list`}
				shortcuts={getListShortcuts(i18n)}
			/>
			<p className="text-sm text-tn-muted">
				{t`All these actions are also available with the buttons.`}
			</p>
		</>
	);
}
