import { ModalProvider } from '@minimalstuff/ui';
import { act, fireEvent, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { DevslotLinkHost } from '#/devslot_link/devslot_link_host';
import { ACTIVATION_DELAY_MS } from '#/devslot_link/use_activation_delay';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

function renderConfirmation() {
	useDaemonStore.setState({ hosts: ['vega'] });
	const gateway = createFakeGateway();
	renderWithGateway(
		<>
			<ModalProvider />
			<DevslotLinkHost />
		</>,
		gateway
	);
	act(() => {
		useUiStore
			.getState()
			.offerDevslotLink({ kind: 'devslotUp', host: 'vega', instance: 'web-1' });
	});
	return gateway;
}

const advance = (milliseconds: number) =>
	act(() => vi.advanceTimersByTimeAsync(milliseconds));

describe('the confirmation activation delay', () => {
	beforeEach(() => {
		vi.useFakeTimers({ shouldAdvanceTime: false });
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('should ignore Open until the delay has passed', async () => {
		const gateway = renderConfirmation();
		await advance(0);

		fireEvent.click(screen.getByRole('button', { name: 'Open' }));
		expect(gateway.devslotUp).not.toHaveBeenCalled();

		await advance(ACTIVATION_DELAY_MS);
		fireEvent.click(screen.getByRole('button', { name: 'Open' }));
		expect(gateway.devslotUp).toHaveBeenCalledTimes(1);
	});

	it('should arm again when the window regains focus', async () => {
		const gateway = renderConfirmation();
		await advance(ACTIVATION_DELAY_MS);

		act(() => void window.dispatchEvent(new Event('focus')));
		fireEvent.click(screen.getByRole('button', { name: 'Open' }));
		expect(gateway.devslotUp).not.toHaveBeenCalled();

		await advance(ACTIVATION_DELAY_MS);
		fireEvent.click(screen.getByRole('button', { name: 'Open' }));
		expect(gateway.devslotUp).toHaveBeenCalledTimes(1);
	});

	it('should let Cancel work at once', async () => {
		renderConfirmation();
		await advance(0);

		fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
		await advance(0);

		expect(useUiStore.getState().devslotLinkRequest).toBeNull();
	});
});
