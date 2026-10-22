const MAX_HORIZONTAL_RADIO_OPTIONS = 2;

/** Radio options sit side by side only when there are few; more are stacked so none gets squeezed. */
export function getRadioOrientation(
	optionCount: number
): 'horizontal' | 'vertical' {
	return optionCount <= MAX_HORIZONTAL_RADIO_OPTIONS
		? 'horizontal'
		: 'vertical';
}
