import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { getForgottenSecretsMessage } from '#/settings/forgotten_secrets_message';

describe('getForgottenSecretsMessage', () => {
	it.each([
		[0, 'No passphrase was remembered'],
		[1, '1 passphrase forgotten'],
		[3, '3 passphrases forgotten'],
	])('should describe %i forgotten passphrases', (count, expected) => {
		expect(getForgottenSecretsMessage(count, i18n)).toBe(expected);
	});
});
