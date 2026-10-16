import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import { App } from '#/app';

describe('App', () => {
	it('should render the main landmark with the app title', () => {
		render(<App />);

		expect(screen.getByRole('main')).toBeInTheDocument();
		expect(
			screen.getByRole('heading', { name: 'Tunneler', level: 1 })
		).toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		const { container } = render(<App />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
