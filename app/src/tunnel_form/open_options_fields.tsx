import { useId, type ChangeEvent } from 'react';
import { Input, RadioOptions } from '@minimalstuff/ui';

import { useDaemonStore } from '#/state/daemon_store';
import type { OpenSchemeDto } from '#/daemon/protocol';
import { buildOpenPreview } from '#/tunnel_form/build_open_preview';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import type { TunnelFormAction } from '#/tunnel_form/tunnel_form_reducer';
import type { TunnelFormErrors } from '#/tunnel_form/validate_tunnel_form';
import {
	OPEN_HOST_OPTIONS,
	describeOpenHost,
} from '#/shared/open_host_options';

const SCHEME_OPTIONS = [
	{ value: 'http', label: 'http' },
	{ value: 'https', label: 'https' },
];
const INHERITED_HOST_VALUE = '';

type OpenOptionsFieldsProps = {
	values: TunnelFormValues;
	errors: TunnelFormErrors;
	dispatch: (action: TunnelFormAction) => void;
};

/** The "Ouverture" fieldset of a local tunnel: how "Ouvrir dans le navigateur" builds its URL. */
export function OpenOptionsFields({
	values,
	errors,
	dispatch,
}: Readonly<OpenOptionsFieldsProps>) {
	const settings = useDaemonStore((state) => state.settings);
	const previewId = useId();
	const preview = buildOpenPreview(values, settings);
	const inheritedHost = describeOpenHost(settings?.openHost ?? 'localhost');
	const hostOptions = [
		{
			value: INHERITED_HOST_VALUE,
			label: `Réglage général (${inheritedHost})`,
		},
		...OPEN_HOST_OPTIONS.map(({ value, label }) => ({ value, label })),
	];

	const changeScheme = (scheme: string) =>
		dispatch({ type: 'openSchemeChanged', scheme: scheme as OpenSchemeDto });
	const changeHost = (host: string) =>
		dispatch({
			type: 'openHostChanged',
			host: host as TunnelFormValues['openHost'],
		});
	const changePath = (event: ChangeEvent<HTMLInputElement>) =>
		dispatch({
			type: 'textChanged',
			field: 'openPath',
			value: event.target.value,
		});

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">Ouverture</legend>
			<RadioOptions
				name="openScheme"
				label="Schéma"
				orientation="horizontal"
				options={SCHEME_OPTIONS}
				value={values.openScheme}
				onChange={changeScheme}
			/>
			<Input
				name="openPath"
				label="Chemin"
				value={values.openPath}
				onChange={changePath}
				error={errors.openPath}
				description="Vide : /"
				placeholder="/"
				autoComplete="off"
			/>
			<RadioOptions
				name="openHost"
				label="Hôte"
				options={hostOptions}
				value={values.openHost}
				onChange={changeHost}
			/>
			{preview !== null && (
				<p id={previewId} className="m-0 text-sm text-tn-muted">
					{preview}
				</p>
			)}
		</fieldset>
	);
}
