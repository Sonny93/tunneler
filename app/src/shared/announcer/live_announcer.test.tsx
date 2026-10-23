import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { LiveAnnouncer } from '#/shared/announcer/live_announcer';

const announce = (
	...args: Parameters<ReturnType<typeof useAnnouncerStore.getState>['announce']>
) => act(() => useAnnouncerStore.getState().announce(...args));

describe('LiveAnnouncer', () => {
	it('should mount an empty status region before any announcement', () => {
		render(<LiveAnnouncer />);

		expect(screen.getByRole('status')).toHaveTextContent('');
	});

	it('should mount an empty alert region before any announcement', () => {
		render(<LiveAnnouncer />);

		expect(screen.getByRole('alert')).toHaveTextContent('');
	});

	it('should put a polite message in the status region', () => {
		render(<LiveAnnouncer />);

		announce('Adonis : Actif', 'polite');

		expect(screen.getByRole('status')).toHaveTextContent('Adonis : Actif');
	});

	it('should put an assertive message in the alert region', () => {
		render(<LiveAnnouncer />);

		announce('Erreur', 'assertive');

		expect(screen.getByRole('alert')).toHaveTextContent('Erreur');
	});

	it('should keep the polite region free of assertive messages', () => {
		render(<LiveAnnouncer />);

		announce('Erreur', 'assertive');

		expect(screen.getByRole('status')).toHaveTextContent('');
	});

	it('should render a new text node when the same message is announced again', () => {
		render(<LiveAnnouncer />);
		announce('Adonis : Actif', 'polite');
		const firstNode = screen.getByText('Adonis : Actif');

		announce('Adonis : Actif', 'polite');

		expect(screen.getByText('Adonis : Actif')).not.toBe(firstNode);
	});

	it('should be visually hidden', () => {
		render(<LiveAnnouncer />);

		expect(screen.getByRole('status')).toHaveClass('sr-only');
	});

	it('should have no axe violations', async () => {
		const { container } = render(<LiveAnnouncer />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
