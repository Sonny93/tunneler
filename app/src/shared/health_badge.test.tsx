import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';

import { TargetHealthBadge } from '#/shared/health_badge';
import { renderWithI18n } from '#/testing/render_with_i18n';

describe('TargetHealthBadge', () => {
	it('should render nothing when the health is unknown', () => {
		const { container } = renderWithI18n(
			<TargetHealthBadge health={{ kind: 'unknown' }} />
		);

		expect(container).toBeEmptyDOMElement();
	});

	it('should show that the target is reachable', () => {
		renderWithI18n(<TargetHealthBadge health={{ kind: 'reachable' }} />);

		expect(screen.getByText('Target reachable')).toBeInTheDocument();
	});

	it('should show that the target is unreachable with its error text', () => {
		renderWithI18n(
			<TargetHealthBadge
				health={{ kind: 'unreachable', since: 0, error: 'connection refused' }}
			/>
		);

		expect(screen.getByText('Target unreachable')).toBeInTheDocument();
		expect(screen.getByText('connection refused')).toBeInTheDocument();
	});

	it('should have no axe violations when unreachable', async () => {
		const { container } = renderWithI18n(
			<TargetHealthBadge
				health={{ kind: 'unreachable', since: 0, error: 'refused' }}
			/>
		);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations when reachable', async () => {
		const { container } = renderWithI18n(
			<TargetHealthBadge health={{ kind: 'reachable' }} />
		);

		expect(await axe(container)).toHaveNoViolations();
	});
});
