import type { CredentialRequestDto } from '#/daemon/protocol';

export type CredentialField = {
	id: string;
	label: string;
	isSecret: boolean;
};

export function getCredentialTitle(
	request: Readonly<CredentialRequestDto>
): string {
	switch (request.kind) {
		case 'passphrase':
			return 'Phrase de passe';
		case 'password':
			return 'Mot de passe';
		case 'keyboardInteractive':
			return request.name === ''
				? `Authentification ${request.alias}`
				: request.name;
	}
}

/** The explanation shown above the inputs; empty when there is nothing to add. */
export function getCredentialDescription(
	request: Readonly<CredentialRequestDto>
): string {
	switch (request.kind) {
		case 'passphrase':
			return `La clé ${request.keyPath} est chiffrée (connexion à ${request.alias}).`;
		case 'password':
			return `Mot de passe de ${request.user}@${request.host} (${request.alias}).`;
		case 'keyboardInteractive':
			return request.instructions;
	}
}

export function getCredentialFields(
	request: Readonly<CredentialRequestDto>
): CredentialField[] {
	switch (request.kind) {
		case 'passphrase':
			return [{ id: 'secret', label: 'Phrase de passe', isSecret: true }];
		case 'password':
			return [{ id: 'secret', label: 'Mot de passe', isSecret: true }];
		case 'keyboardInteractive':
			return request.prompts.map((prompt, position) => ({
				id: `prompt-${position}`,
				label: prompt.text,
				isSecret: !prompt.echo,
			}));
	}
}

export function getCredentialSubmitLabel(
	request: Readonly<CredentialRequestDto>
): string {
	const isWithoutInput =
		request.kind === 'keyboardInteractive' && request.prompts.length === 0;
	return isWithoutInput ? 'Continuer' : 'Valider';
}

/** A string for single-secret requests, the answers in prompt order for keyboard-interactive. */
export function buildCredentialAnswer(
	request: Readonly<CredentialRequestDto>,
	answers: readonly string[]
): string | string[] {
	if (request.kind === 'keyboardInteractive') return [...answers];
	return answers[0] ?? '';
}
