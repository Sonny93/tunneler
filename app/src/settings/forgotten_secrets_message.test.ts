import { describe, expect, it } from 'vitest';

import { getForgottenSecretsMessage } from '#/settings/forgotten_secrets_message';

describe('getForgottenSecretsMessage', () => {
	it.each([
		[0, 'Aucune passphrase mémorisée'],
		[1, '1 passphrase oubliée'],
		[3, '3 passphrases oubliées'],
	])('should describe %i forgotten passphrases', (count, expected) => {
		expect(getForgottenSecretsMessage(count)).toBe(expected);
	});
});
