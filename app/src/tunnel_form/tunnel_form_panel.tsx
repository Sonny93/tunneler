import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { TunnelDefinitionDto } from '#/daemon/protocol';
import { useTunnelForm } from '#/tunnel_form/use_tunnel_form';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { TunnelFormFields } from '#/tunnel_form/tunnel_form_fields';

type TunnelFormPanelProps = {
	tunnel: TunnelDefinitionDto | null;
};

export function TunnelFormPanel({ tunnel }: Readonly<TunnelFormPanelProps>) {
	const formId = useId();
	const closePanel = useUiStore((state) => state.closePanel);
	const { values, errors, isSaving, summary, dispatch, submit } =
		useTunnelForm(tunnel);

	const footer = (
		<div className="flex justify-end gap-2">
			<Button variant="outline" color="neutral" onClick={closePanel}>
				Annuler
			</Button>
			<Button type="submit" form={formId} loading={isSaving}>
				{tunnel ? 'Enregistrer' : 'Créer'}
			</Button>
		</div>
	);

	return (
		<Drawer
			isOpen
			title={tunnel ? `Modifier ${tunnel.name}` : 'Nouveau tunnel'}
			onClose={closePanel}
			footer={footer}
		>
			<form id={formId} noValidate onSubmit={submit}>
				<FormErrorSummary
					messages={summary.messages}
					attach={summary.attachSummary}
				/>
				<TunnelFormFields values={values} errors={errors} dispatch={dispatch} />
			</form>
		</Drawer>
	);
}
