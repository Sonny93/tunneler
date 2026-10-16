import type { Ref } from 'react';
import { Button } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';

type TunnelListEmptyProps = {
	createButtonRef: Ref<HTMLButtonElement>;
};

export function TunnelListEmpty({
	createButtonRef,
}: Readonly<TunnelListEmptyProps>) {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);

	const handleCreate = () => openTunnelEditor(null);

	return (
		<div className="flex flex-col items-start gap-3">
			<p className="m-0 text-tn-muted">Aucun tunnel configuré.</p>
			<Button ref={createButtonRef} onClick={handleCreate}>
				Créer un tunnel
			</Button>
		</div>
	);
}
