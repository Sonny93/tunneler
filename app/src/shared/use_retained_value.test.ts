import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useRetainedValue } from '#/shared/use_retained_value';

const renderRetainedValue = (initialValue: string | null) =>
	renderHook(({ value }: { value: string | null }) => useRetainedValue(value), {
		initialProps: { value: initialValue },
	});

describe('useRetainedValue', () => {
	it('should return null before any value', () => {
		const { result } = renderRetainedValue(null);

		expect(result.current).toBeNull();
	});

	it('should return the current value', () => {
		const { result, rerender } = renderRetainedValue('first');

		rerender({ value: 'second' });

		expect(result.current).toBe('second');
	});

	it('should keep the last value once the value turns null', () => {
		const { result, rerender } = renderRetainedValue('first');

		rerender({ value: null });

		expect(result.current).toBe('first');
	});
});
