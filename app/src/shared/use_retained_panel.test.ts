import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useRetainedPanel } from '#/shared/use_retained_panel';

type TestPanel = { kind: 'x11'; host: string };

const FIRST_PANEL: TestPanel = { kind: 'x11', host: 'nova' };
const SECOND_PANEL: TestPanel = { kind: 'x11', host: 'orion' };

const renderRetainedPanel = (initialPanel: TestPanel | null) =>
	renderHook(
		({ openPanel }: { openPanel: TestPanel | null }) =>
			useRetainedPanel(openPanel),
		{ initialProps: { openPanel: initialPanel } }
	);

describe('useRetainedPanel', () => {
	it('should hold no panel before the first open', () => {
		const { result } = renderRetainedPanel(null);

		expect(result.current).toEqual({
			panel: null,
			isOpen: false,
			instanceKey: 0,
		});
	});

	it('should keep the last panel but report closed once the panel is gone', () => {
		const { result, rerender } = renderRetainedPanel(FIRST_PANEL);

		rerender({ openPanel: null });

		expect(result.current.panel).toBe(FIRST_PANEL);
		expect(result.current.isOpen).toBe(false);
	});

	it('should start a new instance each time a panel opens', () => {
		const { result, rerender } = renderRetainedPanel(null);

		rerender({ openPanel: FIRST_PANEL });
		const firstInstanceKey = result.current.instanceKey;
		rerender({ openPanel: null });
		rerender({ openPanel: { ...FIRST_PANEL } });

		expect(result.current.instanceKey).toBeGreaterThan(firstInstanceKey);
	});

	it('should start a new instance when the open panel is replaced', () => {
		const { result, rerender } = renderRetainedPanel(FIRST_PANEL);
		const firstInstanceKey = result.current.instanceKey;

		rerender({ openPanel: SECOND_PANEL });

		expect(result.current.panel).toBe(SECOND_PANEL);
		expect(result.current.instanceKey).not.toBe(firstInstanceKey);
	});

	it('should keep the same instance while the same panel stays open', () => {
		const { result, rerender } = renderRetainedPanel(FIRST_PANEL);
		const firstInstanceKey = result.current.instanceKey;

		rerender({ openPanel: FIRST_PANEL });

		expect(result.current.instanceKey).toBe(firstInstanceKey);
	});
});
