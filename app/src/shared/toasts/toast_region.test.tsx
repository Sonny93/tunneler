import { act, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { ToastRegion } from '#/shared/toasts/toast_region';
import { useToastStore } from '#/shared/toasts/toast_store';
import { TOAST_AUTO_DISMISS_DELAY_MS } from '#/shared/toasts/use_auto_dismiss';
import { renderWithI18n } from '#/testing/render_with_i18n';

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
		renderWithI18n(<ToastRegion />);

		pushToast({ tone: 'info', message: 'Configuration reloaded' });

		expect(screen.getByRole('status')).toHaveTextContent(
			'Configuration reloaded'
		);
	});

	it('should render an error toast as an alert', () => {
		renderWithI18n(<ToastRegion />);

		pushToast({ tone: 'error', message: 'Failure' });

		expect(screen.getByRole('alert')).toHaveTextContent('Failure');
	});

	it('should dismiss a toast with its close button', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'error', message: 'Failure' });

		await user.click(
			screen.getByRole('button', { name: 'Close notification' })
		);

		expect(screen.queryByRole('alert')).not.toBeInTheDocument();
	});

	it('should auto dismiss an info toast after 5 seconds', () => {
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should auto dismiss a success toast after 5 seconds', () => {
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'success', message: 'Ok' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should keep an error toast until it is dismissed', () => {
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'error', message: 'Failure' });

		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 10));

		expect(screen.getByRole('alert')).toBeInTheDocument();
	});

	it('should pause the timer while the toast is hovered', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		await user.hover(screen.getByRole('status'));
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 2));

		expect(screen.getByRole('status')).toBeInTheDocument();
	});

	it('should pause the timer while the toast has focus', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });

		await user.tab();
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS * 2));

		expect(screen.getByRole('status')).toBeInTheDocument();
	});

	it('should dismiss again after the pointer leaves', async () => {
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });
		await user.hover(screen.getByRole('status'));

		await user.unhover(screen.getByRole('status'));
		void act(() => vi.advanceTimersByTime(TOAST_AUTO_DISMISS_DELAY_MS));

		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('should have no axe violations with toasts of each tone', async () => {
		const { container } = renderWithI18n(<ToastRegion />);
		pushToast({ tone: 'info', message: 'Info' });
		pushToast({ tone: 'success', message: 'Ok' });
		pushToast({ tone: 'error', message: 'Failure' });

		expect(await axe(container)).toHaveNoViolations();
	});
});
