import { clsx, type ClassValue } from 'clsx';

export function cn(...classValues: readonly ClassValue[]): string {
	return clsx(classValues);
}
