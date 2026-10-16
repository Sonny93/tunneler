type FormErrorSummaryProps = {
	messages: readonly string[];
	attach?: (element: HTMLDivElement | null) => void;
};

/** Focusable alert listing why a submission failed; renders nothing without messages. */
export function FormErrorSummary({
	messages,
	attach,
}: Readonly<FormErrorSummaryProps>) {
	if (messages.length === 0) return null;

	return (
		<div
			ref={attach}
			role="alert"
			tabIndex={-1}
			className="mb-4 rounded-md border border-tn-danger bg-tn-danger-subtle p-3 text-sm text-tn-danger"
		>
			<p className="font-semibold">Enregistrement impossible :</p>
			<ul className="list-disc pl-5">
				{messages.map((message) => (
					<li key={message}>{message}</li>
				))}
			</ul>
		</div>
	);
}
