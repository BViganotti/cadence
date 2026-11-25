# Accessibility

Cadence is a local visual engraving workbench. The Studio is keyboard-first and
labels its controls; the score itself is primarily a visual artifact.

## Implemented

- Semantic landmarks: `header`, `main`, `aside`, `section`, `footer`.
- Every toolbar action is a real `button` with a title; file import uses a
  labelled button that drives a hidden file input.
- Form controls have visible labels; selects and inputs are reachable and focus
  visible.
- The score stage is focusable (`tabindex="0"`) with an `aria-label` describing
  how to select notes.
- Inline SVG includes a `<title>` element.
- Hit rectangles are transparent SVG overlays with data attributes; they are
  pointer targets, not text.
- Toast messages use `role="status"` and `aria-live="polite"`.
- Connection state is shown by a dot plus a text label, not colour alone.
- Keyboard shortcuts (ignored while typing in a field):
  - `1`–`7`: choose duration
  - `A`–`G`: enter a note at the selected octave
  - `Delete`/`Backspace`: delete the selected event
  - `ArrowLeft`/`ArrowRight`: move the selected measure
  - `Meta/Ctrl+Z`: undo; `Meta/Ctrl+Shift+Z` or `Ctrl+Y`: redo
- Zoom and page controls are buttons with text labels (`−`, `+`, `‹`, `›`) and
  visible percentages/page numbers.
- Colour contrast targets the dark theme: light text on dark panels, gold
  accents on dark surfaces, white paper score.

## Not implemented

- A screen-reader representation of the score (e.g., a linear note list) is not
  provided. Hit regions expose coordinates and indexes, but a blind user cannot
  currently read the music as text.
- No high-contrast or light theme switch.
- No reduced-motion preference handling for the spinner and smooth page scroll.
- Keyboard operation of the SVG score itself (tabbing between noteheads) is not
  implemented; use the measure list and inspector instead.
- No screen-reader announcement of score changes beyond the status text.

## Testing

The Playwright flow exercises keyboard note entry, keyboard undo/redo, buttons,
imports and exports. DOM labels and landmarks are present for automated
accessibility scanners, but no scanner is bundled because it would require an
external package.
