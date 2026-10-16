import type { ReactNode } from 'react';

type FormHintProps = {
	id: string;
	children: ReactNode;
};

export const FormHint = ({ id, children }: Readonly<FormHintProps>) => (
	<p id={id} className="mt-1 text-xs text-tn-muted">
		{children}
	</p>
);
