import { screen, waitFor } from '@testing-library/react';

type LiveRegionRole = 'alert' | 'status';

/**
 * The library's dialogs and drawers hold their own empty `status` and `alert`
 * live regions, so a role query alone finds several. These helpers keep the
 * regions that carry a message, which are the app's own.
 */
export function getFilledLiveRegions(role: LiveRegionRole): HTMLElement[] {
	return screen
		.queryAllByRole(role)
		.filter((region) => (region.textContent ?? '').trim() !== '');
}

export function getFilledLiveRegion(role: LiveRegionRole): HTMLElement {
	const regions = getFilledLiveRegions(role);
	if (regions.length !== 1) {
		throw new Error(
			`Expected exactly one ${role} region with a message, found ${regions.length}`
		);
	}
	return regions[0];
}

export function findFilledLiveRegion(
	role: LiveRegionRole
): Promise<HTMLElement> {
	return waitFor(() => getFilledLiveRegion(role));
}
