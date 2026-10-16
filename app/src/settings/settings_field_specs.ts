import type { SettingsFormValues } from '#/settings/settings_form_values';

export type SettingsTextField = {
	[TKey in keyof SettingsFormValues]: SettingsFormValues[TKey] extends string
		? TKey
		: never;
}[keyof SettingsFormValues];

export type SettingsFieldSpec = {
	field: SettingsTextField;
	label: string;
	hint?: string;
	inputMode?: 'decimal' | 'numeric';
};

export type SettingsSectionSpec = {
	legend: string;
	fields: readonly SettingsFieldSpec[];
};

/** Text fields in display order; the health probe switch is rendered apart. */
export const SETTINGS_SECTIONS: readonly SettingsSectionSpec[] = [
	{
		legend: 'Adresses',
		fields: [
			{
				field: 'defaultBindAddress',
				label: "Adresse d'écoute par défaut (local)",
			},
			{
				field: 'defaultRemoteBindAddress',
				label: "Adresse d'écoute par défaut (remote)",
			},
		],
	},
	{
		legend: 'Connexion',
		fields: [
			{
				field: 'keepaliveIntervalSeconds',
				label: 'Intervalle de keepalive (secondes)',
				hint: '0 pour désactiver',
				inputMode: 'decimal',
			},
			{
				field: 'keepaliveMax',
				label: 'Keepalives sans réponse tolérés',
				hint: 'Au-delà, la connexion est considérée perdue',
				inputMode: 'numeric',
			},
			{
				field: 'sessionIdleGraceSeconds',
				label: "Délai avant fermeture d'une session inutilisée (secondes)",
				inputMode: 'decimal',
			},
		],
	},
	{
		legend: 'Reconnexion',
		fields: [
			{
				field: 'reconnectInitialDelaySeconds',
				label: 'Délai initial de reconnexion (secondes)',
				inputMode: 'decimal',
			},
			{
				field: 'reconnectMaxDelaySeconds',
				label: 'Délai maximal de reconnexion (secondes)',
				inputMode: 'decimal',
			},
		],
	},
];
