import { useId } from 'react';
import { useLingui } from '@lingui/react/macro';
import type { MessageDescriptor } from '@lingui/core';

import { useDaemonStore } from '#/state/daemon_store';
import type { TunnelKindDto } from '#/daemon/protocol';
import { StandbyField } from '#/tunnel_form/standby_field';
import { TargetPortField } from '#/tunnel_form/target_port_field';
import { TunnelKindField } from '#/tunnel_form/tunnel_kind_field';
import { TunnelTextField } from '#/tunnel_form/tunnel_text_field';
import { OpenOptionsFields } from '#/tunnel_form/open_options_fields';
import { getTunnelKindFieldTexts } from '#/tunnel_form/tunnel_kind_texts';
import type { TunnelFormErrors } from '#/tunnel_form/validate_tunnel_form';
import type {
	StandbyChoice,
	TunnelFormValues,
} from '#/tunnel_form/tunnel_form_values';
import type {
	TunnelFormAction,
	TunnelFormTextField,
} from '#/tunnel_form/tunnel_form_reducer';

type TunnelFormFieldsProps = {
	values: TunnelFormValues;
	errors: TunnelFormErrors;
	dispatch: (action: TunnelFormAction) => void;
	/** Temporary tunnels are never selected by tag, so they have no tags field. */
	hasTags?: boolean;
	/** Temporary tunnels are never opened with saved options. */
	hasOpenOptions?: boolean;
	/** Temporary tunnels start at once, so they never wait in standby. */
	hasStandby?: boolean;
};

export function TunnelFormFields({
	values,
	errors,
	dispatch,
	hasTags = true,
	hasOpenOptions = true,
	hasStandby = true,
}: Readonly<TunnelFormFieldsProps>) {
	const { t, i18n } = useLingui();
	const hosts = useDaemonStore((state) => state.hosts);
	const defaultBindAddress = useDaemonStore((state) =>
		values.kind === 'remote'
			? state.settings?.defaultRemoteBindAddress
			: state.settings?.defaultBindAddress
	);
	const defaultAddressText = defaultBindAddress ?? t`unknown`;
	const texts = getTunnelKindFieldTexts(values.kind);
	const hasTarget = values.kind !== 'dynamic';
	const hostListId = useId();

	const resolveError = (error: MessageDescriptor | undefined) =>
		error && i18n._(error);
	const changeText = (field: TunnelFormTextField, value: string) =>
		dispatch({ type: 'textChanged', field, value });
	const changeKind = (kind: TunnelKindDto) =>
		dispatch({ type: 'kindChanged', kind });
	const changeTargetPort = (value: string) =>
		dispatch({ type: 'targetPortChanged', value });
	const relinkTargetPort = () => dispatch({ type: 'targetPortRelinked' });
	const changeStandby = (choice: StandbyChoice) =>
		dispatch({ type: 'standbyChanged', choice });

	return (
		<div className="flex flex-col gap-4">
			<TunnelKindField value={values.kind} onChange={changeKind} />
			<TunnelTextField
				field="host"
				label={t`Host`}
				value={values.host}
				onChange={changeText}
				error={resolveError(errors.host)}
				hint={t`Alias from ~/.ssh/config`}
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
				label={i18n._(texts.listenPortLabel)}
				value={values.listenPort}
				onChange={changeText}
				error={resolveError(errors.listenPort)}
				inputMode="numeric"
				isRequired
			/>
			{hasTarget && (
				<TargetPortField
					value={values.targetPort}
					isLinked={values.isTargetPortLinked}
					error={resolveError(errors.targetPort)}
					onChange={changeTargetPort}
					onRelink={relinkTargetPort}
				/>
			)}
			{texts.targetHostLabel !== null && texts.targetHostHint !== null && (
				<TunnelTextField
					field="targetHost"
					label={i18n._(texts.targetHostLabel)}
					value={values.targetHost}
					onChange={changeText}
					placeholder="127.0.0.1"
					hint={i18n._(texts.targetHostHint)}
				/>
			)}
			<TunnelTextField
				field="listenAddress"
				label={t`Listen address`}
				value={values.listenAddress}
				onChange={changeText}
				placeholder={defaultBindAddress}
				hint={t`Leave empty to use the default address (${defaultAddressText})`}
			/>
			{hasTags && (
				<TunnelTextField
					field="tags"
					label={t`Tags, separated by commas`}
					value={values.tags}
					onChange={changeText}
					error={resolveError(errors.tags)}
				/>
			)}
			{hasStandby && values.kind !== 'remote' && (
				<StandbyField value={values.standby} onChange={changeStandby} />
			)}
			{hasOpenOptions && values.kind === 'local' && (
				<OpenOptionsFields
					values={values}
					errors={errors}
					dispatch={dispatch}
				/>
			)}
		</div>
	);
}
