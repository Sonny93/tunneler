import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { TunnelFormFields } from '#/tunnel_form/tunnel_form_fields';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import type { TunnelDefinitionDto, TunnelSpecDto } from '#/daemon/protocol';
import {
	useTunnelForm,
	type TunnelFormMode,
} from '#/tunnel_form/use_tunnel_form';

type TunnelFormPanelProps = {
	tunnel: TunnelDefinitionDto | null;
	draft?: TunnelSpecDto;
	mode?: TunnelFormMode;
};

const TEMPORARY_NOTE =
	"Non enregistré : disparaît quand on l'arrête ou quand Tunneler quitte.";

function getTitle(
	tunnel: Readonly<TunnelDefinitionDto> | null,
	draft: Readonly<TunnelSpecDto> | undefined,
	mode: TunnelFormMode
): string {
	if (mode === 'temporary') return 'Nouveau tunnel temporaire';
	if (tunnel) return `Modifier ${getTunnelEndpointLabel(tunnel)}`;
	return draft ? 'Importer un tunnel' : 'Nouveau tunnel';
}

export function TunnelFormPanel({
	tunnel,
	draft,
	mode = 'saved',
}: Readonly<TunnelFormPanelProps>) {
	const formId = useId();
	const closePanel = useUiStore((state) => state.closePanel);
	const { values, errors, isSaving, summary, dispatch, submit } = useTunnelForm(
		tunnel,
		draft ?? null,
		mode
	);
	const isTemporary = mode === 'temporary';
	const submitLabel = tunnel ? 'Enregistrer' : 'Créer';

	const footer = (
		<div className="flex justify-end gap-2">
			<Button variant="outline" color="neutral" onClick={closePanel}>
				Annuler
			</Button>
			<Button type="submit" form={formId} loading={isSaving}>
				{isTemporary ? 'Démarrer' : submitLabel}
			</Button>
		</div>
	);

	return (
		<Drawer
			isOpen
			title={getTitle(tunnel, draft, mode)}
			onClose={closePanel}
			footer={footer}
		>
			<form id={formId} noValidate onSubmit={submit}>
				<FormErrorSummary
					messages={summary.messages}
					attach={summary.attachSummary}
				/>
				{isTemporary && (
					<p className="mt-0 mb-4 text-sm text-tn-muted">{TEMPORARY_NOTE}</p>
				)}
				<TunnelFormFields
					values={values}
					errors={errors}
					dispatch={dispatch}
					hasTags={!isTemporary}
					hasOpenOptions={!isTemporary}
					hasStandby={!isTemporary}
				/>
			</form>
		</Drawer>
	);
}
