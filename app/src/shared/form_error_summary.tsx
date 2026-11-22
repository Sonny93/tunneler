import { useLingui } from '@lingui/react/macro';

import { splitEnglishText } from '#/shared/split_english_text';

type FormErrorSummaryProps = {
	messages: readonly string[];
	/** Parts of the messages that are raw English from the daemon: they get `lang="en"`. */
	englishTexts?: readonly string[];
	title?: string;
	attach?: (element: HTMLDivElement | null) => void;
};

/** Focusable alert listing why a submission failed; renders nothing without messages. */
export function FormErrorSummary({
	messages,
	englishTexts = [],
	title,
	attach,
}: Readonly<FormErrorSummaryProps>) {
	const { t } = useLingui();
	if (messages.length === 0) return null;

	return (
		<div
			ref={attach}
			role="alert"
			tabIndex={-1}
			className="mb-4 rounded-md border border-tn-danger bg-tn-danger-subtle p-3 text-sm text-tn-danger"
		>
			<p className="font-semibold">{title ?? t`Couldn't save:`}</p>
			<ul className="list-disc pl-5">
				{messages.map((message) => (
					<li key={message}>
						{splitEnglishText(message, englishTexts).map((segment) =>
							segment.isEnglish ? (
								<span key={segment.text} lang="en">
									{segment.text}
								</span>
							) : (
								segment.text
							)
						)}
					</li>
				))}
			</ul>
		</div>
	);
}
