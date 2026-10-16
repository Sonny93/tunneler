import { Kbd } from '@minimalstuff/ui';

import {
	GLOBAL_SHORTCUTS,
	LIST_SHORTCUTS,
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

export const ShortcutsHelpContent = () => (
	<>
		<ShortcutGroup title="Partout" shortcuts={GLOBAL_SHORTCUTS} />
		<ShortcutGroup title="Liste des tunnels" shortcuts={LIST_SHORTCUTS} />
		<p className="text-sm text-tn-muted">
			Toutes ces actions sont aussi disponibles avec les boutons.
		</p>
	</>
);
