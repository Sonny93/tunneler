import { useId } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import type { TunnelKindDto } from '#/daemon/protocol';
import { TargetPortField } from '#/tunnel_form/target_port_field';
import { TunnelKindField } from '#/tunnel_form/tunnel_kind_field';
import { TunnelTextField } from '#/tunnel_form/tunnel_text_field';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import { getTunnelKindFieldTexts } from '#/tunnel_form/tunnel_kind_texts';
import type { TunnelFormErrors } from '#/tunnel_form/validate_tunnel_form';
import type {
	TunnelFormAction,
	TunnelFormTextField,
} from '#/tunnel_form/tunnel_form_reducer';

type TunnelFormFieldsProps = {
	values: TunnelFormValues;
	errors: TunnelFormErrors;
	dispatch: (action: TunnelFormAction) => void;
};

export function TunnelFormFields({
	values,
	errors,
	dispatch,
}: Readonly<TunnelFormFieldsProps>) {
	const hosts = useDaemonStore((state) => state.hosts);
	const defaultBindAddress = useDaemonStore((state) =>
		values.kind === 'remote'
			? state.settings?.defaultRemoteBindAddress
			: state.settings?.defaultBindAddress
	);
	const texts = getTunnelKindFieldTexts(values.kind);
	const hasTarget = values.kind !== 'dynamic';
	const hostListId = useId();

	const changeText = (field: TunnelFormTextField, value: string) =>
		dispatch({ type: 'textChanged', field, value });
	const changeKind = (kind: TunnelKindDto) =>
		dispatch({ type: 'kindChanged', kind });
	const changeTargetPort = (value: string) =>
		dispatch({ type: 'targetPortChanged', value });
	const relinkTargetPort = () => dispatch({ type: 'targetPortRelinked' });

	return (
		<div className="flex flex-col gap-4">
			<TunnelKindField value={values.kind} onChange={changeKind} />
			<TunnelTextField
				field="host"
				label="Host"
				value={values.host}
				onChange={changeText}
				error={errors.host}
				hint="Alias de ~/.ssh/config"
				isRequired
				listId={hostListId}
			/>
			<datalist id={hostListId}>
				{hosts.map((host) => (
					<option key={host} value={host}>
						{host}
					</option>
				))}
			</datalist>
			<TunnelTextField
				field="listenPort"
				label={texts.listenPortLabel}
				value={values.listenPort}
				onChange={changeText}
				error={errors.listenPort}
				inputMode="numeric"
				isRequired
			/>
			{hasTarget && (
				<TargetPortField
					value={values.targetPort}
					isLinked={values.isTargetPortLinked}
					error={errors.targetPort}
					onChange={changeTargetPort}
					onRelink={relinkTargetPort}
				/>
			)}
			{hasTarget && (
				<TunnelTextField
					field="targetHost"
					label={texts.targetHostLabel}
					value={values.targetHost}
					onChange={changeText}
					placeholder="127.0.0.1"
					hint={texts.targetHostHint}
				/>
			)}
			<TunnelTextField
				field="listenAddress"
				label="Adresse d'écoute"
				value={values.listenAddress}
				onChange={changeText}
				placeholder={defaultBindAddress}
				hint={`Laisser vide pour utiliser l'adresse par défaut (${defaultBindAddress ?? 'inconnue'})`}
			/>
			<TunnelTextField
				field="tags"
				label="Tags séparés par des virgules"
				value={values.tags}
				onChange={changeText}
				error={errors.tags}
			/>
		</div>
	);
}
