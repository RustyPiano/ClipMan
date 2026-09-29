// Shared UI constants.

/** DOM id of the always-focused QuickBar search input (SearchBar + +page share it). */
export const SEARCH_INPUT_ID = 'quickbar-search';

/**
 * Uniform QuickBar row height in rem. The CSS row height, the virtualization
 * math in +page.svelte and the e2e geometry assertions must all derive from
 * this value; never hardcode it elsewhere.
 */
export const ROW_HEIGHT_REM = 4.5;
