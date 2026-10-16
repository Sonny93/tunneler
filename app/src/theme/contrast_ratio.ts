const HEX_PATTERN = /^#([0-9a-f]{6})$/i;
const CHANNEL_MAXIMUM = 255;
const LINEARIZATION_THRESHOLD = 0.03928;
const LUMINANCE_OFFSET = 0.05;
const RED_WEIGHT = 0.2126;
const GREEN_WEIGHT = 0.7152;
const BLUE_WEIGHT = 0.0722;

function linearizeChannel(channelValue: number): number {
	const normalized = channelValue / CHANNEL_MAXIMUM;
	if (normalized <= LINEARIZATION_THRESHOLD) return normalized / 12.92;
	return ((normalized + 0.055) / 1.055) ** 2.4;
}

/** WCAG 2.x relative luminance of a `#rrggbb` color. */
export function getRelativeLuminance(hexColor: string): number {
	const match = HEX_PATTERN.exec(hexColor);
	if (!match?.[1])
		throw new Error(`Expected a #rrggbb color, got "${hexColor}"`);
	const packed = Number.parseInt(match[1], 16);
	const red = linearizeChannel((packed >> 16) & 0xff);
	const green = linearizeChannel((packed >> 8) & 0xff);
	const blue = linearizeChannel(packed & 0xff);
	return RED_WEIGHT * red + GREEN_WEIGHT * green + BLUE_WEIGHT * blue;
}

/** WCAG 2.x contrast ratio between two `#rrggbb` colors, from 1 to 21. */
export function getContrastRatio(
	firstColor: string,
	secondColor: string
): number {
	const firstLuminance = getRelativeLuminance(firstColor);
	const secondLuminance = getRelativeLuminance(secondColor);
	const lighter = Math.max(firstLuminance, secondLuminance);
	const darker = Math.min(firstLuminance, secondLuminance);
	return (lighter + LUMINANCE_OFFSET) / (darker + LUMINANCE_OFFSET);
}
