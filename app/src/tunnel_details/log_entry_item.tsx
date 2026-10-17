import { formatTime } from '#/shared/format_time';
import type { LogEntryDto } from '#/daemon/protocol';
import { LOG_LEVEL_APPEARANCES } from '#/tunnel_details/log_level_appearance';

type LogEntryItemProps = {
	entry: LogEntryDto;
};

/** Engine messages are English, hence the `lang` on the message. */
export function LogEntryItem({ entry }: Readonly<LogEntryItemProps>) {
	const { icon, label, toneClassName } = LOG_LEVEL_APPEARANCES[entry.level];

	return (
		<li className="flex flex-wrap items-baseline gap-x-2 text-xs">
			<time
				dateTime={new Date(entry.at).toISOString()}
				className="font-mono text-tn-muted"
			>
				{formatTime(entry.at)}
			</time>
			<span className={`inline-flex items-center gap-1 ${toneClassName}`}>
				<span aria-hidden="true" className={`${icon} size-4 shrink-0`} />
				{label}
			</span>
			<span lang="en" className="min-w-0 break-words">
				{entry.message}
			</span>
		</li>
	);
}
