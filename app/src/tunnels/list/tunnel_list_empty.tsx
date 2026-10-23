import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import type { Ref } from 'react';

import { useUiStore } from '#/state/ui_store';

type TunnelListEmptyProps = {
	createButtonRef: Ref<HTMLButtonElement>;
};

export function TunnelListEmpty({
	createButtonRef,
}: Readonly<TunnelListEmptyProps>) {
	const { t } = useLingui();
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);

	const handleCreate = () => openTunnelEditor(null);

	return (
		<div className="flex flex-col items-start gap-3">
			<p className="m-0 text-tn-muted">{t`No tunnels yet.`}</p>
			<Button ref={createButtonRef} onClick={handleCreate}>
				{t`Create a tunnel`}
			</Button>
		</div>
	);
}
