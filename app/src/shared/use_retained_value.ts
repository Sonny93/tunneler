import { useState } from 'react';

/** Keeps returning the last non-null value once `value` turns null, so a closing drawer keeps its content while it slides out. */
export function useRetainedValue<T>(value: T | null): T | null {
	const [retained, setRetained] = useState<T | null>(value);
	if (value !== null && value !== retained) setRetained(value);
	return value ?? retained;
}
