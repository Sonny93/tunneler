import type { I18n, MessageDescriptor } from '@lingui/core';

type MessageOption<TValue extends string> = {
	value: TValue;
	label: MessageDescriptor;
};

/** Resolves the `msg` labels of module-level option lists, for the lib's `Select` and radio options. */
export function translateOptions<TValue extends string>(
	options: readonly MessageOption<TValue>[],
	i18n: I18n
): { value: TValue; label: string }[] {
	return options.map(({ value, label }) => ({ value, label: i18n._(label) }));
}
