import { describe, expect, it } from 'vitest';

import { buildThemeColors, buildThemeCss } from './theme_css';
import { THEME_TOKENS } from './tokens';

describe('buildThemeCss', () => {
	it('should declare the light tokens on :root', () => {
		expect(buildThemeCss()).toContain(
			`:root { color-scheme: light; --tn-bg: ${THEME_TOKENS.light.background};`
		);
	});

	it('should declare the dark tokens on the class the component library toggles', () => {
		expect(buildThemeCss()).toContain(
			`.dark { color-scheme: dark; --tn-bg: ${THEME_TOKENS.dark.background};`
		);
	});

	it('should disable motion for users who prefer reduced motion', () => {
		expect(buildThemeCss()).toContain(
			'@media (prefers-reduced-motion: reduce)'
		);
	});

	it('should never scroll the window itself, only its regions', () => {
		expect(buildThemeCss()).toContain('html, body { overflow: hidden; }');
	});

	it('should keep a border on status badges in forced colors mode', () => {
		expect(buildThemeCss()).toContain(
			'.tn-status-badge { border: 1px solid CanvasText; }'
		);
	});
});

describe('buildThemeColors', () => {
	it('should map color names to CSS variables', () => {
		expect(buildThemeColors()).toMatchObject({
			'tn-bg': 'var(--tn-bg)',
			'tn-muted': 'var(--tn-muted)',
			'tn-success-subtle': 'var(--tn-success-subtle)',
			'tn-focus-ring': 'var(--tn-focus-ring)',
			'tn-divider': 'var(--tn-divider)',
			'tn-selected': 'var(--tn-selected)',
		});
	});
});
