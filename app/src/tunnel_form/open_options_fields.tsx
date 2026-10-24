import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { Input, RadioOptions, Select } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import type { AvailableOpenerDto, OpenSchemeDto } from '#/daemon/protocol';
import { getBrowserChoice, BROWSER_OPENER_ID } from '#/openers/open_choices';
import {
	describeOpenHost,
	getOpenHostOptions,
} from '#/shared/open_host_options';
import { getRadioOrientation } from '#/shared/radio_orientation';
import { useDaemonStore } from '#/state/daemon_store';
import { buildOpenPreview } from '#/tunnel_form/build_open_preview';
import { suggestOpener } from '#/tunnel_form/suggest_opener';
import type { TunnelFormAction } from '#/tunnel_form/tunnel_form_reducer';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import type { TunnelFormErrors } from '#/tunnel_form/validate_tunnel_form';

const SCHEME_OPTIONS = [
	{ value: 'http', label: 'http' },
	{ value: 'https', label: 'https' },
];
const INHERITED_HOST_VALUE = '';

const AUTOMATIC_VALUE = '';

function buildApplicationOptions(
	selected: string,
	suggestedName: string,
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): { value: string; label: string }[] {
	const isMissing =
		selected !== AUTOMATIC_VALUE &&
		selected !== BROWSER_OPENER_ID &&
		openers.every(({ id }) => id !== selected);
	return [
		{
			value: AUTOMATIC_VALUE,
			label: i18n._(msg`Automatic (${suggestedName})`),
		},
		{ value: BROWSER_OPENER_ID, label: getBrowserChoice(i18n).name },
		...openers.map(({ id, name }) => ({ value: id, label: name })),
		...(isMissing
			? [{ value: selected, label: i18n._(msg`${selected} (unavailable)`) }]
			: []),
	];
}

type OpenOptionsFieldsProps = {
	values: TunnelFormValues;
	errors: TunnelFormErrors;
	dispatch: (action: TunnelFormAction) => void;
};

/** The "Opening" fieldset of a local tunnel: how "Open in the browser" builds its URL. */
export function OpenOptionsFields({
	values,
	errors,
	dispatch,
}: Readonly<OpenOptionsFieldsProps>) {
	const { t, i18n } = useLingui();
	const settings = useDaemonStore((state) => state.settings);
	const openers = useDaemonStore((state) => state.openers);
	const previewId = useId();
	const preview = buildOpenPreview(values, settings, openers, i18n);
	const suggested = suggestOpener(
		values,
		openers,
		settings?.openDefault ?? 'suggested'
	);
	const applicationOptions = buildApplicationOptions(
		values.openWith,
		suggested?.name ?? getBrowserChoice(i18n).name,
		openers,
		i18n
	);
	const inheritedHost = describeOpenHost(
		settings?.openHost ?? 'localhost',
		i18n
	);
	const hostOptions = [
		{
			value: INHERITED_HOST_VALUE,
			label: t`General setting (${inheritedHost})`,
		},
		...getOpenHostOptions(i18n).map(({ value, label }) => ({ value, label })),
	];

	const changeScheme = (scheme: string) =>
		dispatch({ type: 'openSchemeChanged', scheme: scheme as OpenSchemeDto });
	const changeHost = (host: string) =>
		dispatch({
			type: 'openHostChanged',
			host: host as TunnelFormValues['openHost'],
		});
	const changeApplication = (event: ChangeEvent<HTMLSelectElement>) =>
		dispatch({ type: 'openWithChanged', opener: event.target.value });
	const changePath = (event: ChangeEvent<HTMLInputElement>) =>
		dispatch({
			type: 'textChanged',
			field: 'openPath',
			value: event.target.value,
		});

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Opening`}</legend>
			<RadioOptions
				name="openScheme"
				label={t`Scheme`}
				orientation={getRadioOrientation(SCHEME_OPTIONS.length)}
				options={SCHEME_OPTIONS}
				value={values.openScheme}
				onChange={changeScheme}
			/>
			<Select
				name="openWith"
				label={t`Application`}
				options={applicationOptions}
				value={values.openWith}
				onChange={changeApplication}
			/>
			<Input
				name="openPath"
				label={t`Path`}
				value={values.openPath}
				onChange={changePath}
				error={errors.openPath && i18n._(errors.openPath)}
				description={t`Empty: /`}
				placeholder="/"
				autoComplete="off"
			/>
			<RadioOptions
				name="openHost"
				label={t`Host`}
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
