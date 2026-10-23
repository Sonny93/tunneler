import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';

/** Toast text for the number of remembered passphrases that were erased. */
export function getForgottenSecretsMessage(
	forgottenCount: number,
	i18n: I18n
): string {
	return i18n._(
		msg`${plural(forgottenCount, {
			0: 'No passphrase was remembered',
			one: '# passphrase forgotten',
			other: '# passphrases forgotten',
		})}`
	);
}
