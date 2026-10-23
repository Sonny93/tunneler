import { screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { AppLayout } from '#/app_layout';
import { renderWithI18n } from '#/testing/render_with_i18n';

const renderLayout = (isSidebarCollapsed = false) =>
	renderWithI18n(
		<AppLayout
			sidebar={<nav aria-label="Tags">Tags</nav>}
			sidebarToggle={<button>Sidebar</button>}
			isSidebarCollapsed={isSidebarCollapsed}
			actions={<button>Create</button>}
		>
			<p>Content</p>
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
			screen.getByRole('button', { name: 'Create' })
		);
	});

	it('should render the sidebar in a labelled complementary landmark', () => {
		renderLayout();

		expect(
			screen.getByRole('complementary', { name: 'Navigation' })
		).toHaveTextContent('Tags');
	});

	it('should render the sidebar toggle in the banner', () => {
		renderLayout();

		expect(screen.getByRole('banner')).toContainElement(
			screen.getByRole('button', { name: 'Sidebar' })
		);
	});

	it('should hide the sidebar when collapsed', () => {
		renderLayout(true);

		expect(
			screen.queryByRole('complementary', { name: 'Navigation' })
		).not.toBeInTheDocument();
	});

	it('should render the children in the main landmark', () => {
		renderLayout();

		expect(screen.getByRole('main')).toHaveTextContent('Content');
	});

	it('should offer a skip link to the main region', () => {
		renderLayout();

		const skipLink = screen.getByRole('link', {
			name: 'Skip to the tunnel list',
		});
		expect(skipLink).toHaveAttribute('href', `#${screen.getByRole('main').id}`);
	});

	it('should have no axe violations', async () => {
		const { container } = renderLayout();

		expect(await axe(container)).toHaveNoViolations();
	});
});
