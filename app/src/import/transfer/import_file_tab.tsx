import { ConfigExportSection } from '#/import/transfer/config_export_section';
import { ConfigImportSection } from '#/import/transfer/config_import_section';

export function ImportFileTab() {
	return (
		<div className="flex flex-col gap-6 pt-3">
			<ConfigExportSection />
			<ConfigImportSection />
		</div>
	);
}
