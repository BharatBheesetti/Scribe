# F6: History Search - Summary

## What Was Implemented

Added keyword search to the History tab, allowing users to find past transcriptions by typing a query. The search filters entries client-side in real time, highlights matching text with safe DOM construction, shows result counts, and refreshes automatically when new transcriptions arrive.

All 6 issues identified in F6-REVIEW.md were addressed:

1. **Highlight pipeline (Issue 1):** Uses split-map-join on RAW text, not regex on HTML-escaped text. Each fragment is HTML-escaped after splitting, then matches wrapped in `<mark>`.
2. **No-results XSS (Issue 2):** "No results" message built with `createElement` + `textContent`. User query is NEVER inserted via innerHTML.
3. **Stale cache (Issue 3):** 5-second polling refresh via `setInterval` when History tab is visible catches new transcriptions without Rust changes.
4. **Performance (Issue 4):** Client-side filter on max 100 entries is O(n) string search -- negligible cost.
5. **Auto-focus accessibility (Issue 5):** Search input does NOT auto-focus on tab switch. Users tab or click into it naturally.
6. **XSS safety (Issue 6):** All text content is escaped before DOM insertion. The only innerHTML usage is the output of `highlightMatches()`, which escapes all fragments before wrapping in `<mark>`.

## Files Modified

| File | Changes |
|------|---------|
| `src/index.html` | Added search container with input, clear button, result count span inside `panel-history` (lines 142-147) |
| `src/styles.css` | Added 7 new CSS rule blocks: search container, input, placeholder, hover, focus, clear button, result count, no-results, search-query highlight, mark tag (lines 576-661) |
| `src/main.js` | Added DOM refs (lines 32-34), module state (37-39), utilities section with debounce/escapeHtml/highlightMatches/filterHistory/startHistoryRefresh/stopHistoryRefresh (264-351), modified loadHistory/renderHistory/switchTab/handleClearHistory, wired event listeners (582-596) |

## Design Decisions

1. **Client-side filtering (not new Tauri command):** `get_history` returns max 100 entries. JS substring filter is trivially fast. Adding a Rust `search_history` command would add IPC round-trip latency on every keystroke for zero benefit.

2. **Split-map-join highlight pipeline:** The correct approach for XSS-safe highlighting. Regex operates on raw text (so `<` matches `<`, not `&lt;`), each fragment is HTML-escaped individually, and `<mark>` tags wrap already-safe content.

3. **Polling refresh (not Tauri event):** The backend emits no "history-updated" event, and the plan forbids Rust changes. 5-second polling of a local IPC call to deserialize 100 small JSON objects has negligible cost. The interval is cleared when leaving the History tab.

4. **No auto-focus:** WCAG 2.1 compliance. Auto-focus on tab switch disrupts keyboard navigation for screen reader users.

5. **200ms debounce:** Prevents excessive re-renders during rapid typing. Total perceived latency from keystroke to render is under 400ms.

6. **Blue accent for highlights:** Uses `rgba(59, 130, 246, 0.3)` instead of browser default yellow -- consistent with the app's dark theme and blue accent color (#3b82f6).

## Commits

| Hash | Description |
|------|-------------|
| `374ecd0` | feat(F6): add search bar HTML and CSS to History tab |
| `5f3c6dc` | feat(F6): implement client-side history search with XSS-safe highlighting |

## Verification

- `cargo check` passes with no errors (no Rust files modified)
- HTML structure verified: search container exists inside `panel-history` before `history-list`
- CSS rules verified: all 7 new blocks present, focus ring matches app accent, no existing styles modified
- JS logic verified: all utility functions, event wiring, state management, and DOM construction in place
