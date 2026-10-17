import type { ImportWarningDto } from '#/daemon/protocol';

/** The French sentence around the raw English text (`raw`), which is shown apart in the page. */
export type ImportWarningText = {
	before: string;
	raw: string;
	after: string;
};

function describeLinePrefix(line: number | undefined): string {
	return line === undefined ? '' : `Ligne ${line} : `;
}

export function describeImportWarning(
	warning: Readonly<ImportWarningDto>
): ImportWarningText {
	switch (warning.kind) {
		case 'malformedForward':
			return {
				before: `Ligne ${warning.line} : option ignorée, illisible : `,
				raw: warning.text,
				after: '',
			};
		case 'unsupportedForward': {
			const hostPrefix = warning.host === undefined ? '' : `${warning.host} : `;
			return {
				before: `${describeLinePrefix(warning.line)}${hostPrefix}forward non pris en charge (socket Unix ou SOCKS distant) : `,
				raw: warning.text,
				after: '',
			};
		}
		case 'unknownDestination':
			return {
				before: 'Aucun alias ssh_config ne correspond à ',
				raw: warning.destination,
				after: " : choisissez l'hôte ci-dessous.",
			};
		case 'hostResolutionFailed':
			return {
				before: `Lecture de ${warning.host} impossible : `,
				raw: warning.message,
				after: '',
			};
	}
}
