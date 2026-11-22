import type { ConfigImportText } from '#/import/transfer/describe_config_import';

type ConfigImportTextViewProps = {
	text: ConfigImportText;
};

/** The sentence with its raw English part, if any, marked as English. */
export const ConfigImportTextView = ({
	text,
}: Readonly<ConfigImportTextViewProps>) => (
	<>
		{text.before}
		{text.raw !== '' && <span lang="en">{text.raw}</span>}
		{text.after}
	</>
);
