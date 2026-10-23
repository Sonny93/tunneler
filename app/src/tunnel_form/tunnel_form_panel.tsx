import { useLingui } from '@lingui/react/macro';
import { Button, Drawer } from '@minimalstuff/ui';
import { useId } from 'react';

import type { TunnelDefinitionDto, TunnelSpecDto } from '#/daemon/protocol';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useUiStore } from '#/state/ui_store';
import { TunnelFormFields } from '#/tunnel_form/tunnel_form_fields';
import {
	useTunnelForm,
	type TunnelFormMode,
} from '#/tunnel_form/use_tunnel_form';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type TunnelFormPanelProps = {
	tunnel: TunnelDefinitionDto | null;
	draft?: TunnelSpecDto;
	mode?: TunnelFormMode;
	isOpen: boolean;
};

export function TunnelFormPanel({
	tunnel,
	draft,
	mode = 'saved',
	isOpen,
}: Readonly<TunnelFormPanelProps>) {
	const { t } = useLingui();
	const formId = useId();
	const closePanel = useUiStore((state) => state.closePanel);
	const { values, errors, isSaving, summary, dispatch, submit } = useTunnelForm(
		tunnel,
		draft ?? null,
		mode
	);
	const isTemporary = mode === 'temporary';
	const submitLabel = tunnel ? t`Save` : t`Create`;

	const getTitle = (): string => {
		if (mode === 'temporary') return t`New temporary tunnel`;
		if (tunnel) {
			const endpointLabel = getTunnelEndpointLabel(tunnel);
			return t`Edit ${endpointLabel}`;
		}
		return draft ? t`Import a tunnel` : t`New tunnel`;
	};

	const footer = (
		<div className="flex justify-end gap-2">
			<Button variant="outline" color="neutral" onClick={closePanel}>
				{t`Cancel`}
			</Button>
			<Button type="submit" form={formId} loading={isSaving}>
				{isTemporary ? t`Start` : submitLabel}
			</Button>
		</div>
	);

	return (
		<Drawer
			isOpen={isOpen}
			title={getTitle()}
			onClose={closePanel}
			footer={footer}
		>
			<form id={formId} noValidate onSubmit={submit}>
				<FormErrorSummary
					messages={summary.messages}
					attach={summary.attachSummary}
				/>
				{isTemporary && (
					<p className="mt-0 mb-4 text-sm text-tn-muted">{t`Not saved: it disappears when it is stopped or when Tunneler quits.`}</p>
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
