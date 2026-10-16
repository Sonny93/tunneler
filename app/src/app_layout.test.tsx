import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import { AppLayout } from '#/app_layout';

const renderLayout = () =>
	render(
		<AppLayout
			sidebar={<nav aria-label="Tags">Tags</nav>}
			actions={<button>Créer</button>}
		>
			<p>Contenu</p>
		</AppLayout>
	);

describe('AppLayout', () => {
	it('should render the title as a level 1 heading', () => {
		renderLayout();

		expect(
			screen.getByRole('heading', { name: 'Tunneler', level: 1 })
		).toBeInTheDocument();
	});

	it('should render the actions in the banner', () => {
		renderLayout();

		expect(screen.getByRole('banner')).toContainElement(
			screen.getByRole('button', { name: 'Créer' })
		);
	});

	it('should render the sidebar in a labelled complementary landmark', () => {
		renderLayout();

		expect(
			screen.getByRole('complementary', { name: 'Navigation' })
		).toHaveTextContent('Tags');
	});

	it('should render the children in the main landmark', () => {
		renderLayout();

		expect(screen.getByRole('main')).toHaveTextContent('Contenu');
	});

	it('should offer a skip link to the main region', () => {
		renderLayout();

		const skipLink = screen.getByRole('link', {
			name: 'Aller à la liste des tunnels',
		});
		expect(skipLink).toHaveAttribute('href', `#${screen.getByRole('main').id}`);
	});

	it('should have no axe violations', async () => {
		const { container } = renderLayout();

		expect(await axe(container)).toHaveNoViolations();
	});
});
