import { useLingui } from '@lingui/react/macro';
import { Button, Input, Textarea } from '@minimalstuff/ui';
import {
	useEffect,
	useRef,
	useState,
	type ChangeEvent,
	type KeyboardEvent,
} from 'react';

import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';
import {
	EMPTY_OPENER_FORM_VALUES,
	PLACEHOLDER_TEXTS,
	slugifyName,
	toOpenerFormValues,
	toOpenerSpec,
	type OpenerFormValues,
} from '#/openers/opener_form_values';
import {
	getFirstInvalidOpenerField,
	validateOpenerForm,
	type OpenerFormErrors,
	type OpenerFormField,
} from '#/openers/validate_opener_form';
import { areRecordsEqual } from '#/shared/are_records_equal';

type OpenerFormProps = {
	/** The application being edited, or `null` to add one. */
	opener: Readonly<OpenerSpecDto> | null;
	/** Prefills the form of an addition, e.g. from a detected application to customize. */
	draft?: Readonly<OpenerSpecDto>;
	/** The detected applications: one with the same identifier is replaced by the saved one. */
	detectedOpeners: readonly Readonly<AvailableOpenerDto>[];
	/** The identifiers of the user's other applications. */
	takenIds: readonly string[];
	onSubmit: (spec: OpenerSpecDto) => void;
	onCancel: () => void;
	/** Tells whether the fields differ from the ones the form opened with. */
	onInputChange: (hasInput: boolean) => void;
};

function focusField(container: HTMLElement, field: OpenerFormField): void {
	container.querySelector<HTMLElement>(`[name="opener-${field}"]`)?.focus();
}

export function OpenerForm({
	opener,
	draft,
	detectedOpeners,
	takenIds,
	onSubmit,
	onCancel,
	onInputChange,
}: Readonly<OpenerFormProps>) {
	const { t, i18n } = useLingui();
	const containerRef = useRef<HTMLFieldSetElement>(null);
	const { hostPlaceholder, portPlaceholder, urlPlaceholder } =
		PLACEHOLDER_TEXTS;
	const isCreation = opener === null;
	const openerName = opener?.name ?? '';
	const formLabel = isCreation ? t`Add an application` : t`Edit ${openerName}`;
	const initialOpener = opener ?? draft;
	const [initialValues] = useState<OpenerFormValues>(
		initialOpener ? toOpenerFormValues(initialOpener) : EMPTY_OPENER_FORM_VALUES
	);
	const [values, setValues] = useState(initialValues);
	const hasInput = !areRecordsEqual(values, initialValues);
	const [isIdEdited, setIsIdEdited] = useState(draft !== undefined);
	const replacedName = detectedOpeners.find(
		({ id }) => id === values.id.trim()
	)?.name;
	const [errors, setErrors] = useState<OpenerFormErrors>({});

	useEffect(() => onInputChange(hasInput), [hasInput, onInputChange]);

	const change = (field: keyof OpenerFormValues, value: string) => {
		setErrors(({ [field]: _edited, ...others }) => others);
		setValues((current) => ({ ...current, [field]: value }));
	};
	const handleName = (event: ChangeEvent<HTMLInputElement>) => {
		const name = event.target.value;
		change('name', name);
		if (isCreation && !isIdEdited) change('id', slugifyName(name));
	};
	const handleId = (event: ChangeEvent<HTMLInputElement>) => {
		setIsIdEdited(true);
		change('id', event.target.value);
	};
	const handleText =
		(field: 'command' | 'ports') => (event: ChangeEvent<HTMLInputElement>) =>
			change(field, event.target.value);
	const handleArguments = (event: ChangeEvent<HTMLTextAreaElement>) =>
		change('args', event.target.value);

	const idDescription = isCreation
		? t`Lowercase letters, digits and dashes. Cannot be changed after creation.`
		: t`Cannot be changed: tunnels refer to it by this identifier.`;

	const submit = () => {
		const foundErrors = validateOpenerForm(values, takenIds);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidOpenerField(foundErrors);
		if (firstInvalidField !== null && containerRef.current !== null) {
			focusField(containerRef.current, firstInvalidField);
			return;
		}
		onSubmit(toOpenerSpec(values));
	};

	const handleEnter = (event: KeyboardEvent<HTMLInputElement>) => {
		if (event.key !== 'Enter') return;
		event.preventDefault();
		submit();
	};

	return (
		<fieldset
			ref={containerRef}
			data-opener-form
			aria-label={formLabel}
			className="m-0 flex min-w-0 flex-col gap-4 rounded-md border border-tn-border p-3"
		>
			<Input
				name="opener-name"
				onKeyDown={handleEnter}
				label={t`Name`}
				value={values.name}
				onChange={handleName}
				error={errors.name && i18n._(errors.name)}
				required
				autoComplete="off"
			/>
			<Input
				name="opener-id"
				onKeyDown={handleEnter}
				label={t`Identifier`}
				value={values.id}
				onChange={handleId}
				error={errors.id && i18n._(errors.id)}
				description={
					replacedName === undefined
						? idDescription
						: t`Replaces the detected ${replacedName}.`
				}
				readOnly={!isCreation}
				required
				autoComplete="off"
			/>
			<Input
				name="opener-command"
				onKeyDown={handleEnter}
				label={t`Command`}
				value={values.command}
				onChange={handleText('command')}
				error={errors.command && i18n._(errors.command)}
				description={t`The program to run, without its arguments: a name found in the PATH, or the full path of the executable.`}
				required
				autoComplete="off"
			/>
			<Textarea
				name="opener-args"
				label={t`Arguments`}
				value={values.args}
				onChange={handleArguments}
				error={errors.args && i18n._(errors.args)}
				description={t`One argument per line, passed as is: no shell interprets quotes or spaces. ${hostPlaceholder} is the address the tunnel listens on (localhost or its IP), ${portPlaceholder} is the local port of the tunnel (not the target port), ${urlPlaceholder} is the full address, as opened in the browser.`}
				rows={4}
			/>
			<Input
				name="opener-ports"
				onKeyDown={handleEnter}
				label={t`Suggested ports`}
				value={values.ports}
				onChange={handleText('ports')}
				error={errors.ports && i18n._(errors.ports)}
				description={t`Separated by commas. A tunnel whose target port is one of them offers this application by default.`}
				inputMode="numeric"
				autoComplete="off"
			/>
			<div className="flex justify-end gap-2">
				<Button variant="outline" color="neutral" onClick={onCancel}>
					{t`Cancel`}
				</Button>
				<Button onClick={submit}>{isCreation ? t`Add` : t`Update`}</Button>
			</div>
		</fieldset>
	);
}
