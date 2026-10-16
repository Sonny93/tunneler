import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import { TargetHealthBadge } from '#/shared/health_badge';

describe('TargetHealthBadge', () => {
	it('should render nothing when the health is unknown', () => {
		const { container } = render(
			<TargetHealthBadge health={{ kind: 'unknown' }} />
		);

		expect(container).toBeEmptyDOMElement();
	});

	it('should show that the target is reachable', () => {
		render(<TargetHealthBadge health={{ kind: 'reachable' }} />);

		expect(screen.getByText('Cible joignable')).toBeInTheDocument();
	});

	it('should show that the target is unreachable', () => {
		render(
			<TargetHealthBadge
				health={{ kind: 'unreachable', since: 0, error: 'connection refused' }}
			/>
		);

		expect(screen.getByText('Cible injoignable')).toBeInTheDocument();
	});

	it('should include the error text of an unreachable target', () => {
		render(
			<TargetHealthBadge
				health={{ kind: 'unreachable', since: 0, error: 'connection refused' }}
			/>
		);

		expect(screen.getByText('connection refused')).toBeInTheDocument();
	});

	it('should have no axe violations when unreachable', async () => {
		const { container } = render(
			<TargetHealthBadge
				health={{ kind: 'unreachable', since: 0, error: 'refused' }}
			/>
		);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations when reachable', async () => {
		const { container } = render(
			<TargetHealthBadge health={{ kind: 'reachable' }} />
		);

		expect(await axe(container)).toHaveNoViolations();
	});
});
