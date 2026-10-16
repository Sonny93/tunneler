import { axe } from 'vitest-axe';
import userEvent from '@testing-library/user-event';
import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ToastRegion } from '#/shared/toasts/toast_region';
import { useToastStore } from '#/shared/toasts/toast_store';
import { TOAST_AUTO_DISMISS_DELAY_MS } from '#/shared/toasts/use_auto_dismiss';

const pushToast = (
	...args: Parameters<ReturnType<typeof useToastStore.getState>['pushToast']>
) => act(() => useToastStore.getState().pushToast(...args));

describe('ToastRegion', () => {
	beforeEach(() => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('should render the message of a pushed toast', () => {
		render(<ToastRegion />);

		pushToast({ tone: 'info', message: 'Configuration rechargée' });

		expect(screen.getByRole('status')).toHaveTextContent(
			'Configuration rechargée'
		);
	});

	it('should render an error toast as an alert', () => {
		render(<ToastRegion />);

		pushToast({ tone: 'error', message: 'Échec' });

		expect(screen.getByRole('alert')).toHaveTextContent('Échec');
	});

	it('should dismiss a toast with its close button', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		render(<ToastRegion />);
		pushToast({ tone: 'error', message: 'Échec' });

		await user.click(
			screen.getByRole('button', { name: 'Fermer la notification' })
		);

		expect(screen.queryByRole('alert')).not.toBeInTheDocument();
	});

	it('should auto dismiss an info toast after 5 seconds', () => {
		render(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should auto dismiss a success toast after 5 seconds', () => {
		render(<ToastRegion />);
		pushToast({ tone: 'success', message: 'Ok' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should keep an error toast until it is dismissed', () => {
		render(<ToastRegion />);
		pushToast({ tone: 'error', message: 'Échec' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 10));

		expect(screen.getByRole('alert')).toBeInTheDocument();
	});

	it('should pause the timer while the toast is hovered', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		render(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		await user.hover(screen.getByRole('status'));
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 2));

		expect(screen.getByRole('status')).toBeInTheDocument();
	});

	it('should pause the timer while the toast has focus', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		render(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		await user.tab();
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 2));

		expect(screen.getByRole('status')).toBeInTheDocument();
	});

	it('should dismiss again after the pointer leaves', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		render(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });
		await user.hover(screen.getByRole('status'));

		await user.unhover(screen.getByRole('status'));
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should have no axe violations with toasts of each tone', async () => {
		const { container } = render(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });
		pushToast({ tone: 'success', message: 'Ok' });
		pushToast({ tone: 'error', message: 'Échec' });

		expect(await axe(container)).toHaveNoViolations();
	});
});
