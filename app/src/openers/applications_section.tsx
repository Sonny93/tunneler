import { useId } from 'react';
import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { OpenerForm } from '#/openers/opener_form';
import type { AvailableOpenerDto } from '#/daemon/protocol';
import { useOpenersEditor } from '#/openers/use_openers_editor';

function describeCommandLine(opener: Readonly<AvailableOpenerDto>): string {
	return [opener.command, ...opener.args]
		.map((part) => (/\s/u.test(part) ? `"${part}"` : part))
		.join(' ');
}

type OpenerListProps = {
	headingId: string;
	openers: readonly AvailableOpenerDto[];
	children?: (opener: Readonly<AvailableOpenerDto>) => React.ReactNode;
};

function OpenerList({
	headingId,
	openers,
	children,
}: Readonly<OpenerListProps>) {
	return (
		<ul
			aria-labelledby={headingId}
			className="m-0 flex list-none flex-col gap-2 p-0"
		>
			{openers.map((opener) => (
				<li
					key={opener.id}
					className="flex flex-wrap items-center justify-between gap-2"
				>
					<div className="min-w-0">
						<div className="text-sm font-medium">{opener.name}</div>
						<code className="block break-all font-mono text-xs text-tn-muted">
							{describeCommandLine(opener)}
						</code>
					</div>
					{children?.(opener)}
				</li>
			))}
		</ul>
	);
}

type UserOpenerActionsProps = {
	opener: Readonly<AvailableOpenerDto>;
	onEdit: (id: string) => void;
	onRemove: (opener: Readonly<AvailableOpenerDto>) => Promise<void>;
};

function UserOpenerActions({
	opener,
	onEdit,
	onRemove,
}: Readonly<UserOpenerActionsProps>) {
	const { t } = useLingui();
	const { name } = opener;

	return (
		<div className="flex gap-1">
			<Button
				size="xs"
				variant="outline"
				color="neutral"
				aria-label={t`Edit ${name}`}
				onClick={() => onEdit(opener.id)}
			>
				{t`Edit`}
			</Button>
			<Button
				size="xs"
				variant="outline"
				color="danger"
				aria-label={t`Delete ${name}`}
				onClick={() => void onRemove(opener)}
			>
				{t`Delete`}
			</Button>
		</div>
	);
}

/** The "Applications" section of the settings: detected ones read-only, the user's ones editable. */
export function ApplicationsSection() {
	const { t } = useLingui();
	const headingId = useId();
	const detectedId = useId();
	const userId = useId();
	const editor = useOpenersEditor();
	const { editing, userOpeners } = editor;
	const editedOpener =
		editing.kind === 'edit'
			? userOpeners.find(({ id }) => id === editing.id)
			: undefined;
	const takenIds = userOpeners
		.map(({ id }) => id)
		.filter((id) => id !== editedOpener?.id);

	return (
		<section aria-labelledby={headingId} className="mt-6 flex flex-col gap-4">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{t`Applications`}
			</h3>
			<p className="m-0 text-xs text-tn-muted">
				{t`They open a tunnel instead of the browser. Commands run without a shell, with the arguments as is.`}
			</p>
			<div className="flex flex-col gap-2">
				<h4 id={detectedId} className="m-0 text-xs font-semibold">
					{t`Detected on this computer`}
				</h4>
				{editor.detectedOpeners.length === 0 ? (
					<p className="m-0 text-xs text-tn-muted">
						{t`No application detected.`}
					</p>
				) : (
					<OpenerList headingId={detectedId} openers={editor.detectedOpeners} />
				)}
			</div>
			<div className="flex flex-col gap-2">
				<h4 id={userId} className="m-0 text-xs font-semibold">
					{t`Added`}
				</h4>
				{userOpeners.length === 0 ? (
					<p className="m-0 text-xs text-tn-muted">
						{t`No application added.`}
					</p>
				) : (
					<OpenerList headingId={userId} openers={userOpeners}>
						{(opener) => (
							<UserOpenerActions
								opener={opener}
								onEdit={editor.startEditing}
								onRemove={editor.remove}
							/>
						)}
					</OpenerList>
				)}
			</div>
			{editing.kind === 'none' && (
				<div>
					<Button
						startIcon="i-mdi-plus"
						variant="outline"
						color="neutral"
						onClick={editor.startCreating}
					>
						{t`Add an application`}
					</Button>
				</div>
			)}
			{editing.kind !== 'none' && (
				<OpenerForm
					key={editedOpener?.id ?? 'new'}
					opener={editedOpener ?? null}
					takenIds={takenIds}
					onSubmit={editor.save}
					onCancel={editor.stopEditing}
				/>
			)}
		</section>
	);
}
