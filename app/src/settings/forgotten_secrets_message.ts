/** French toast text for the number of remembered passphrases that were erased. */
export function getForgottenSecretsMessage(forgottenCount: number): string {
	if (forgottenCount === 0) return 'Aucune passphrase mémorisée';
	if (forgottenCount === 1) return '1 passphrase oubliée';
	return `${forgottenCount} passphrases oubliées`;
}
