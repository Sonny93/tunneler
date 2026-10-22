const UNEXPECTED_CODE = 'unexpected';

export class DaemonCommandError extends Error {
	readonly code: string;
	readonly data: unknown;

	constructor(code: string, message: string, data?: unknown) {
		super(message);
		this.name = 'DaemonCommandError';
		this.code = code;
		this.data = data;
	}
}

type NativeRejection = { code: string; message: string; data?: unknown };

function isNativeRejection(rejection: unknown): rejection is NativeRejection {
	if (typeof rejection !== 'object' || rejection === null) return false;
	return (
		'code' in rejection &&
		typeof rejection.code === 'string' &&
		'message' in rejection &&
		typeof rejection.message === 'string'
	);
}

function describeUnknown(rejection: unknown): string {
	if (rejection instanceof Error) return rejection.message;
	if (typeof rejection === 'string') return rejection;
	return 'Unexpected error';
}

export function toDaemonCommandError(rejection: unknown): DaemonCommandError {
	if (rejection instanceof DaemonCommandError) return rejection;
	if (!isNativeRejection(rejection)) {
		return new DaemonCommandError(UNEXPECTED_CODE, describeUnknown(rejection));
	}
	return new DaemonCommandError(
		rejection.code,
		rejection.message,
		rejection.data
	);
}
