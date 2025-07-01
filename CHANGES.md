# Changes Made to Transcript View

## Summary
Modified the transcript view to change the "x lines hidden" text format and added "expand all" functionality.

## Changes Made

### 1. TranscriptView.tsx
- **Modified interface**: Added optional `expandAll` prop to `TranscriptViewProps`
- **Updated text format**: Changed from `"x lines hidden"` to `"[X lines above/below] or [expand all]"`
- **Added interactivity**: 
  - Made the line count text clickable to expand context (existing functionality)
  - Added clickable "[expand all]" text to show all lines for that transcript
- **Added function**: `handleExpandAll()` function to call the expandAll prop when clicked

### 2. App.tsx
- **Added function**: `expandAll(filename: string)` function that clears the visible lines filter for a specific file, which causes all lines to be displayed
- **Updated component usage**: Added `expandAll={expandAll}` prop to the TranscriptView component

## How it works

1. **Line count display**: Shows `[X lines above]` or `[X lines below]` depending on the direction, with hover effects
2. **Expand context**: Clicking the line count adds 16 lines in the specified direction (existing functionality)
3. **Expand all**: Clicking "[expand all]" removes the line filtering for that transcript, showing all lines
4. **Visual feedback**: Both clickable elements have hover effects (blue color and underline)

## Technical Details

- When `visibleLines` is empty for a file, all lines are shown by default
- The `expandAll` function sets the `searchLineNumbers` for the specific filename to an empty array `[]`
- This triggers a re-render where `visibleLines` becomes empty, causing all transcript blocks to be visible
- The text format uses conditional rendering to show "above" or "below" based on the direction data