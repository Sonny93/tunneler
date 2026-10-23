# Accessibility

The UI targets WCAG 2.2 level AA. The command line follows the `NO_COLOR` and non-terminal conventions. A change to the UI or the CLI is not done until these rules hold.

## Definition of done

- It can be used with the keyboard alone.
- Every control has a correct accessible name.
- No information is carried by colour alone.
- UI: `axe` reports no violation. CLI: the output is correct with `NO_COLOR=1` and without a terminal.

## UI

### Keyboard
- Every action is reachable without a mouse, in a logical tab order.
- The focus ring is always visible, with at least 3:1 contrast.
- Lists use arrow-key navigation (roving tabindex): one tab stop per list.
- Shortcuts are listed in the help (`?`) and are never the only way to do something: each action also has a button.

### Structure
- Use native HTML first: `button`, `form`, `label`, `table`, lists.
- Use landmarks (`nav`, `main`, `aside`) and a proper heading hierarchy.
- Complex ARIA patterns (modal, menu, combobox, switch, tabs, tooltip) come from `@minimalstuff/ui`. If the library falls short on accessibility, fix it in the library instead of working around it in the app.

### Names and states
- A control says what it does and its state: a tunnel switch reads "Tunnel 3333 to nova:3333, active", not "on".
- Icon-only buttons always have a label.
- Every state or health value is an icon plus text, never a colour alone ("Reconnecting 2", "Target unreachable").
- Text contrast is at least 4.5:1 and component contrast at least 3:1, in both themes.

### Announcements
- State changes go through a polite `aria-live` region, grouped and throttled so that starting a tag gives one announcement, not twenty. Nothing is announced at load.
- Assertive is for blocking errors only.
- Prompts (passphrase, host key) take the focus.
- The detail drawer's log is silent (`aria-live="off"`).

### Modals
- Focus is trapped inside, Escape closes, and focus returns to the element that opened it.
- The exception is the changed host key alert: it needs an explicit choice, so Escape does nothing.

### Forms
- Visible labels.
- Errors are linked to their field (`aria-describedby`, `aria-invalid`), say precisely what is wrong, and do not depend on hover.
- In a form split into tabs, each tab with errors says so in its name ("Connection 2 errors"), never by colour alone. A failed submit opens the first tab with an error and focuses the field.

### System preferences
- `prefers-reduced-motion`: no animation is essential.
- `prefers-color-scheme`: both themes work.
- `forced-colors` (Windows high contrast): badges, focus and icons stay visible.
- 200% zoom or large text: no loss of content and no horizontal scrolling, down to the minimum window size.

### Targets
Clickable areas are at least 24 by 24 px (WCAG 2.5.8).

## Tray
- Menu items are text and include the state ("dev: 3/5 active"). Tags are checkable.
- The tooltip summarises the global state, and each state has a distinct icon shape.
- The tray is never the only way: everything it does is available in the UI and the CLI.

## Command line
- State is a symbol plus a word (`✔ active`, `✖ failed`), with an ASCII fallback when the terminal is not UTF-8.
- Honour `NO_COLOR`, `TERM=dumb` and `--color auto|always|never`. Colour, spinners and line rewriting turn off by themselves without a terminal.
- A spinner is never the only progress signal: print a line for each step.
- Default output reads line by line, with no box-drawing tables that screen readers garble. `--json` is stable for scripts.
- Errors go to stderr with the cause and a suggested action. Exit codes are documented.
- Interactive confirmations can be skipped (`--yes`), and nothing prompts without a terminal.
- Every subcommand has a complete `--help` with examples.

## Testing

Automated tests cover `axe` on every component and screen, roles and accessible names, focus in modals and drawers, token contrast in both themes, and the CLI output with `NO_COLOR=1` and without a terminal. They do not see a real rendering engine, a real screen reader or the system preferences, so those are checked by hand.

| | Linux | Windows |
|---|---|---|
| Webview | WebKitGTK | WebView2 |
| Screen reader | Orca | NVDA |

Test the release binaries, not `pnpm tauri dev`. Open manual checks are listed on the [roadmap](/roadmap/#needs-testing).
