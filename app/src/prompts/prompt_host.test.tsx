import { ModalProvider } from '@minimalstuff/ui';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { CredentialRequestDto, EventMessage } from '#/daemon/protocol';
import { PromptHost } from '#/prompts/prompt_host';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { buildSettings } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

const passphraseRequest: CredentialRequestDto = {
	kind: 'passphrase',
	alias: 'nova',
	keyPath: '~/.ssh/id_ed25519',
};

const credentialEvent = (
	promptId: string,
	request: CredentialRequestDto
): EventMessage => ({
	event: 'credential.request',
	data: { promptId, ...request },
});

const hostKeyEvent = (promptId: string): EventMessage => ({
	event: 'hostkey.request',
	data: {
		kind: 'prompt',
		promptId,
		alias: 'nova',
		lookupName: '[nova.example.com]:2222',
		algorithm: 'ssh-ed25519',
		fingerprintSha256: 'SHA256:abcdef',
	},
});

const alertEvent: EventMessage = {
	event: 'hostkey.request',
	data: {
		kind: 'alert',
		alias: 'nova',
		presentedFingerprint: 'SHA256:new',
		reason: {
			kind: 'changed',
			knownFingerprint: 'SHA256:known',
			path: '/home/jules/.ssh/known_hosts',
			line: 12,
		},
	},
};

const findSecretInput = () =>
	screen.findByLabelText('Passphrase', { selector: 'input' });

function renderHost(gateway = createFakeGateway()) {
	const rendered = renderWithGateway(
		<>
			<ModalProvider />
			<PromptHost />
		</>,
		gateway
	);
	const receive = (event: EventMessage) =>
		act(() => useDaemonStore.getState().applyEvent(event));
	return { ...rendered, receive };
}

describe('PromptHost credential prompts', () => {
	it('should ask for the passphrase with a focused hidden input and the default retention choices', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));

		expect(
			await screen.findByRole('dialog', { name: 'Passphrase' })
		).toHaveTextContent(
			'The key ~/.ssh/id_ed25519 is encrypted (connecting to nova).'
		);
		const secretInput = await findSecretInput();
		expect(secretInput).toHaveFocus();
		expect(secretInput).toHaveAttribute('type', 'password');
		expect(
			screen.getByRole('radio', { name: 'Until Tunneler closes' })
		).toBeChecked();
		expect(
			screen.queryByRole('radio', { name: 'In the system keyring' })
		).not.toBeInTheDocument();
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should send the passphrase and dismiss the prompt', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 's3cret');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				's3cret',
				'session'
			)
		);
		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should send the passphrase with the submit button', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'abc');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				'abc',
				'session'
			)
		);
	});

	it('should send null when cancelled with the button', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.click(await screen.findByRole('button', { name: 'Cancel' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				null,
				undefined
			)
		);
	});

	it('should send null when closed with Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				null,
				undefined
			)
		);
		expect(gateway.respondCredential).toHaveBeenCalledOnce();
	});

	it('should show the keyring option when the keyring is enabled', async () => {
		useDaemonStore.setState({
			settings: buildSettings({ useOsKeyring: true }),
		});
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));

		expect(
			await screen.findByRole('radio', { name: 'In the system keyring' })
		).toBeInTheDocument();
	});

	it('should send keyring retention when chosen', async () => {
		useDaemonStore.setState({
			settings: buildSettings({ useOsKeyring: true }),
		});
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'abc');
		await user.click(
			screen.getByRole('radio', { name: 'In the system keyring' })
		);
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				'abc',
				'keyring'
			)
		);
	});

	it('should send no retention when asked not to remember', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'abc');
		await user.click(screen.getByRole('radio', { name: "Don't remember" }));
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				'abc',
				undefined
			)
		);
	});

	it('should have no axe violations on a passphrase prompt', async () => {
		useDaemonStore.setState({
			settings: buildSettings({ useOsKeyring: true }),
		});
		const { receive } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await findSecretInput();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should describe a password request and not offer retention', async () => {
		const { receive } = renderHost();

		receive(
			credentialEvent('p1', {
				kind: 'password',
				alias: 'nova',
				user: 'jules',
				host: 'nova.example.com',
			})
		);

		expect(
			await screen.findByRole('dialog', { name: 'Password' })
		).toHaveTextContent('Password for jules@nova.example.com (nova).');
		expect(screen.queryByRole('radio')).not.toBeInTheDocument();
	});

	it('should send keyboard-interactive answers in prompt order', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: 'Duo',
				instructions: 'Enter your codes',
				prompts: [
					{ text: 'Code', echo: true },
					{ text: 'PIN', echo: false },
				],
			})
		);

		await user.type(await screen.findByLabelText('Code'), '123');
		await user.type(screen.getByLabelText('PIN'), '9876');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				['123', '9876'],
				undefined
			)
		);
	});

	it('should show the instructions and echo only echoing prompts', async () => {
		const { receive } = renderHost();

		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: 'Duo',
				instructions: 'Enter your codes',
				prompts: [
					{ text: 'Code', echo: true },
					{ text: 'PIN', echo: false },
				],
			})
		);

		expect(await screen.findByText('Enter your codes')).toBeInTheDocument();
		expect(screen.getByLabelText('Code')).toHaveAttribute('type', 'text');
		expect(screen.getByLabelText('PIN')).toHaveAttribute('type', 'password');
	});

	it('should only offer to continue without prompts', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: '',
				instructions: '',
				prompts: [],
			})
		);

		expect(
			await screen.findByRole('dialog', { name: 'Authentication for nova' })
		).toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Continue' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith(
				'p1',
				[],
				undefined
			)
		);
	});

	it('should stay silent when the prompt is no longer pending', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.respondCredential.mockRejectedValueOnce(
			new DaemonCommandError('prompt_not_pending', 'plus en attente')
		);
		const { receive } = renderHost(gateway);
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'x');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() => expect(useDaemonStore.getState().prompts).toEqual([]));
		expect(useToastStore.getState().toasts).toEqual([]);
	});

	it('should toast other errors and still dismiss the prompt', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.respondCredential.mockRejectedValueOnce(
			new DaemonCommandError('io_error', 'channel closed')
		);
		const { receive } = renderHost(gateway);
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'x');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				"Couldn't send the answer: channel closed"
			)
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should close the dialog without answering when another client resolved it', async () => {
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await screen.findByRole('dialog');

		receive({ event: 'prompt.resolved', data: { promptId: 'p1' } });

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(gateway.respondCredential).not.toHaveBeenCalled();
	});

	it('should show the next prompt once the first one is answered', async () => {
		const user = userEvent.setup();
		const { receive } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		receive(hostKeyEvent('p2'));
		await user.type(await findSecretInput(), 'x');
		await user.click(screen.getByRole('button', { name: 'Submit' }));

		expect(
			await screen.findByRole('dialog', { name: 'Unknown host' })
		).toBeInTheDocument();
	});

	it('should show one dialog at a time', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));
		receive(hostKeyEvent('p2'));

		expect(await screen.findAllByRole('dialog')).toHaveLength(1);
	});
});

describe('PromptHost host key prompts', () => {
	it('should show the key details, offer to copy the fingerprint and focus the refusal button', async () => {
		const { receive } = renderHost();

		receive(hostKeyEvent('p1'));

		const dialog = await screen.findByRole('dialog', { name: 'Unknown host' });
		for (const text of [
			'nova',
			'[nova.example.com]:2222',
			'ssh-ed25519',
			'SHA256:abcdef',
			'Check this fingerprint with the server administrator before accepting it.',
		]) {
			expect(dialog).toHaveTextContent(text, { normalizeWhitespace: true });
		}
		expect(
			screen.getByRole('button', { name: 'Copy the fingerprint' })
		).toBeInTheDocument();
		await waitFor(() =>
			expect(screen.getByRole('button', { name: 'Reject' })).toHaveFocus()
		);
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it.each([
		['Accept and save', 'acceptAndSave'],
		['Accept once', 'acceptOnce'],
		['Reject', 'reject'],
	])('should send the decision of "%s"', async (label, decision) => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(hostKeyEvent('p1'));

		await user.click(await screen.findByRole('button', { name: label }));

		await waitFor(() =>
			expect(gateway.respondHostKey).toHaveBeenCalledWith('p1', decision)
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should reject on Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(hostKeyEvent('p1'));
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(gateway.respondHostKey).toHaveBeenCalledWith('p1', 'reject')
		);
	});
});

describe('PromptHost host key alerts', () => {
	it('should explain the changed key in an alert dialog that only offers to acknowledge it', async () => {
		const { receive } = renderHost();

		receive(alertEvent);

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Host key changed',
		});
		for (const text of [
			'Connection to nova refused.',
			'SHA256:new',
			'SHA256:known',
			'/home/jules/.ssh/known_hosts:12',
			'delete line 12 of /home/jules/.ssh/known_hosts and try again.',
		]) {
			expect(dialog).toHaveTextContent(text, { normalizeWhitespace: true });
		}
		expect(
			screen.getAllByRole('button').map((button) => button.textContent)
		).toEqual(['Got it']);
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should not close on Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(alertEvent);
		await screen.findByRole('alertdialog');

		await user.keyboard('{Escape}');

		expect(screen.getByRole('alertdialog')).toBeInTheDocument();
		expect(gateway.respondHostKey).not.toHaveBeenCalled();
	});

	it('should close on acknowledgement without answering the daemon', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(alertEvent);

		await user.click(await screen.findByRole('button', { name: 'Got it' }));

		await waitFor(() =>
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
		expect(gateway.respondHostKey).not.toHaveBeenCalled();
	});
});
