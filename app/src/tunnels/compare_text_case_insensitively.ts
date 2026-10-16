export function compareTextCaseInsensitively(
	left: string,
	right: string
): number {
	return left.localeCompare(right, 'fr', { sensitivity: 'base' });
}
