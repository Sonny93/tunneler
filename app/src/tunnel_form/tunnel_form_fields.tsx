import { useId } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { TargetPortField } from '#/tunnel_form/target_port_field';
import { TunnelTextField } from '#/tunnel_form/tunnel_text_field';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
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
	const defaultBindAddress = useDaemonStore(
		(state) => state.settings?.defaultBindAddress
	);
	const hostListId = useId();

	const changeText = (field: TunnelFormTextField, value: string) =>
		dispatch({ type: 'textChanged', field, value });
	const changeTargetPort = (value: string) =>
		dispatch({ type: 'targetPortChanged', value });
	const relinkTargetPort = () => dispatch({ type: 'targetPortRelinked' });

	return (
		<div className="flex flex-col gap-4">
			<TunnelTextField
				field="name"
				label="Nom"
				value={values.name}
				onChange={changeText}
				error={errors.name}
				isRequired
			/>
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
				label="Port d'écoute"
				value={values.listenPort}
				onChange={changeText}
				error={errors.listenPort}
				inputMode="numeric"
				isRequired
			/>
			<TargetPortField
				value={values.targetPort}
				isLinked={values.isTargetPortLinked}
				error={errors.targetPort}
				onChange={changeTargetPort}
				onRelink={relinkTargetPort}
			/>
			<TunnelTextField
				field="targetHost"
				label="Hôte cible"
				value={values.targetHost}
				onChange={changeText}
				placeholder="127.0.0.1"
				hint="Vu depuis le serveur SSH"
			/>
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
			/>
		</div>
	);
}
