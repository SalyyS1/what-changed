# Timeline and search

**Evidence state: available; query and virtualization measurements live in
`docs/PERFORMANCE.md`.**

Timeline pages are bounded and keyset-paginated. Filters can narrow by time,
category, importance and entity kind. Search accepts words and returns grouped
events/checkpoints without exposing the raw database to the webview.

Use the live pill to distinguish a currently updating view from a completed
page. A quiet or empty result is a valid state, not an error. Details are
progressive: the summary is visible first and technical evidence is optional.
