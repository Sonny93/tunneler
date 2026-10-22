import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { TagRenameHost } from '#/tag_rename/tag_rename_host';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { useUiStore, type TagRenameOrigin } from '#/state/ui_store';

function renderHost(
	gateway = createFakeGateway(),
	origin: TagRenameOrigin = 'list'
) {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'a', tags: ['dev'] }),
			buildTunnel({ id: 'b', tags: ['dev', 'prod'] }),
		],
	});
	useUiStore.getState().openTagRename('dev', origin);
	return renderWithGateway(
		<>
			<ModalProvider />
			<TagRenameHost />
		</>,
		gateway
	);
}

const findField = () => screen.findByRole('textbox', { name: 'New tag name' });

async function submitName(
	user: ReturnType<typeof userEvent.setup>,
	name: string
) {
	const field = await findField();
	await user.clear(field);
	if (name !== '') await user.type(field, name);
	await user.click(screen.getByRole('button', { name: 'Rename' }));
}

describe('TagRenameHost', () => {
	it('should open a dialog with the current name prefilled', async () => {
		renderHost();

		expect(
			await screen.findByRole('dialog', { name: 'Rename tag dev' })
		).toBeInTheDocument();
		expect(await findField()).toHaveValue('dev');
	});

	it('should rename the tag, toast and close', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.renameTag.mockResolvedValue(2);
		renderHost(gateway);

		await submitName(user, ' staging ');

		await waitFor(() =>
			expect(gateway.renameTag).toHaveBeenCalledWith('dev', 'staging')
		);
		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'success',
					message: 'Tag dev renamed to staging (2 tunnels)',
				}),
			])
		);
		expect(useUiStore.getState().tagRename).toBeNull();
	});

	it('should switch the active filter to the new name', async () => {
		const user = userEvent.setup();
		renderHost();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });

		await submitName(user, 'staging');

		await waitFor(() =>
			expect(useUiStore.getState().filter).toEqual({
				kind: 'tag',
				tag: 'staging',
			})
		);
	});

	it('should keep another filter untouched', async () => {
		const user = userEvent.setup();
		renderHost();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'prod' });

		await submitName(user, 'staging');

		await waitFor(() => expect(useUiStore.getState().tagRename).toBeNull());
		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'prod' });
	});

	it('should ask the new row to take the focus unless the trigger survives', async () => {
		const user = userEvent.setup();
		renderHost(createFakeGateway(), 'sidebar');

		await submitName(user, 'staging');

		await waitFor(() =>
			expect(useUiStore.getState().pendingFocusTag).toBe('staging')
		);
	});

	it('should not move the focus when the trigger is in the list', async () => {
		const user = userEvent.setup();
		renderHost();

		await submitName(user, 'staging');

		await waitFor(() => expect(useUiStore.getState().tagRename).toBeNull());
		expect(useUiStore.getState().pendingFocusTag).toBeNull();
	});

	it.each([
		['', 'Enter a tag name.'],
		['a,b', 'Only one tag, without a comma.'],
		['a b', 'The tag cannot contain spaces.'],
	])('should reject "%s" without calling the daemon', async (name, message) => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderHost(gateway);

		await submitName(user, name);

		expect(await findField()).toHaveAccessibleDescription(message);
		expect(screen.getAllByRole('alert')[0]).toHaveTextContent(message);
		expect(gateway.renameTag).not.toHaveBeenCalled();
	});

	it('should warn that an existing tag will be merged', async () => {
		const user = userEvent.setup();
		renderHost();
		const field = await findField();

		await user.clear(field);
		await user.type(field, 'prod');

		expect(field).toHaveAccessibleDescription(
			'The tag prod already exists: the two will be merged.'
		);
	});

	it('should show the daemon validation messages on the field', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.renameTag.mockRejectedValue(
			new DaemonCommandError('validation_failed', 'invalid', ['Tag rejected'])
		);
		renderHost(gateway);

		await submitName(user, 'staging');

		expect((await screen.findAllByRole('alert'))[0]).toHaveTextContent(
			'Tag rejected'
		);
		expect(useToastStore.getState().toasts).toEqual([]);
	});

	it('should toast other errors and keep the dialog open', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.renameTag.mockRejectedValue(
			new DaemonCommandError('not_found', 'No tunnel')
		);
		renderHost(gateway);

		await submitName(user, 'staging');

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: "Couldn't rename the tag: No tunnel",
				}),
			])
		);
		expect(screen.getByRole('dialog')).toBeInTheDocument();
	});

	it('should close without renaming on cancel', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderHost(gateway);
		await findField();

		await user.click(screen.getByRole('button', { name: 'Cancel' }));

		await waitFor(() => expect(useUiStore.getState().tagRename).toBeNull());
		expect(gateway.renameTag).not.toHaveBeenCalled();
	});

	it('should have no axe violations', async () => {
		renderHost();
		const dialog = await screen.findByRole('dialog');

		expect(await axe(dialog)).toHaveNoViolations();
	});
});
