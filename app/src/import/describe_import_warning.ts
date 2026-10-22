import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { ImportWarningDto } from '#/daemon/protocol';

/** The sentence around the raw text (`raw`), which is shown apart in the page. */
export type ImportWarningText = {
	before: string;
	raw: string;
	after: string;
};

function describeUnsupportedForward(
	warning: Readonly<Extract<ImportWarningDto, { kind: 'unsupportedForward' }>>,
	i18n: I18n
): string {
	const { line, host } = warning;
	if (line !== undefined && host !== undefined) {
		return t(
			i18n
		)`Line ${line}: ${host}: unsupported forward (Unix socket or remote SOCKS): `;
	}
	if (line !== undefined) {
		return t(
			i18n
		)`Line ${line}: unsupported forward (Unix socket or remote SOCKS): `;
	}
	if (host !== undefined) {
		return t(
			i18n
		)`${host}: unsupported forward (Unix socket or remote SOCKS): `;
	}
	return t(i18n)`Unsupported forward (Unix socket or remote SOCKS): `;
}

export function describeImportWarning(
	warning: Readonly<ImportWarningDto>,
	i18n: I18n
): ImportWarningText {
	switch (warning.kind) {
		case 'malformedForward': {
			const { line } = warning;
			return {
				before: t(i18n)`Line ${line}: option ignored, unreadable: `,
				raw: warning.text,
				after: '',
			};
		}
		case 'unsupportedForward':
			return {
				before: describeUnsupportedForward(warning, i18n),
				raw: warning.text,
				after: '',
			};
		case 'unknownDestination':
			return {
				before: t(i18n)`No ssh_config alias matches `,
				raw: warning.destination,
				after: t(i18n)`: choose the host below.`,
			};
		case 'hostResolutionFailed': {
			const { host } = warning;
			return {
				before: t(i18n)`Couldn't read ${host}: `,
				raw: warning.message,
				after: '',
			};
		}
	}
}
