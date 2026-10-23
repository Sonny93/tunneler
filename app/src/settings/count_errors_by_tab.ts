import {
	CONNECTION_SECTIONS,
	STANDBY_SECTION,
	getSettingsFieldTab,
} from '#/settings/settings_field_specs';
import type { SettingsTab } from '#/settings/settings_tab';
import type { SettingsFormErrors } from '#/settings/validate_settings_form';

export type SettingsTabErrorCounts = Readonly<
	Partial<Record<SettingsTab, number>>
>;

const VALIDATED_FIELDS = [...CONNECTION_SECTIONS, STANDBY_SECTION].flatMap(
	(section) => section.fields.map((spec) => spec.field)
);

/** How many invalid fields each tab holds; tabs without errors are absent. */
export function countErrorsByTab(
	errors: Readonly<SettingsFormErrors>
): SettingsTabErrorCounts {
	const counts: Partial<Record<SettingsTab, number>> = {};
	for (const field of VALIDATED_FIELDS) {
		if (!errors[field]) continue;
		const tab = getSettingsFieldTab(field);
		counts[tab] = (counts[tab] ?? 0) + 1;
	}
	return counts;
}
