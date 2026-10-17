import type { LogLevelDto } from '#/daemon/protocol';

type LogLevelAppearance = {
	icon: string;
	label: string;
	toneClassName: string;
};

export const LOG_LEVEL_APPEARANCES: Readonly<
	Record<LogLevelDto, LogLevelAppearance>
> = {
	info: {
		icon: 'i-mdi-information-outline',
		label: 'Info',
		toneClassName: 'text-tn-info',
	},
	warn: {
		icon: 'i-mdi-alert-outline',
		label: 'Avertissement',
		toneClassName: 'text-tn-warning',
	},
	error: {
		icon: 'i-mdi-alert-octagon-outline',
		label: 'Erreur',
		toneClassName: 'text-tn-danger',
	},
};
