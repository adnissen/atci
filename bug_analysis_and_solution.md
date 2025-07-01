# Watch Directory Configuration Bug Analysis and Solution

## Problem Description

When a user changes the watch directory configuration value via the UI, the files from the old watch directory continue to be displayed until the page is manually refreshed. This creates a confusing user experience where the UI shows outdated information.

## Root Cause Analysis

After analyzing the codebase, I've identified the root cause of this bug:

### Current Flow:
1. User changes watch directory in ConfigSetup component
2. Configuration is saved to backend via POST `/config`
3. ConfigSetup calls `onConfigComplete()` which triggers `handleConfigUpdate()` in App.tsx
4. `handleConfigUpdate()` only updates the `watchDirectory` state by fetching from `/watch_directory` endpoint
5. **The file list is never refreshed** - the `files` state remains unchanged
6. User sees old files until page refresh triggers a new `/files` request

### Key Issue:
The `handleConfigUpdate()` function in App.tsx (lines 510-520) only updates the watch directory path but doesn't refresh the file list:

```typescript
const handleConfigUpdate = async () => {
  // Refresh the watch directory after config update
  try {
    const response = await fetch('/watch_directory')
    if (response.ok) {
      const data = await response.text()
      setWatchDirectory(data)
    }
  } catch (error) {
    console.error('Error fetching updated watch directory:', error)
  }
  // Close the config editor
  setIsConfigEditorOpen(false)
}
```

**Missing**: A call to `refreshFiles()` to update the file list from the new directory.

## Backend Behavior Analysis

The backend correctly handles the directory change:
- `/files` endpoint in `TranscriptController.get_mp4_files/0` reads from `ConfigManager.get_config_value("watch_directory")`
- When config is updated, the ConfigManager immediately reflects the new directory
- Subsequent `/files` requests return files from the new directory

## Solution

### Proposed Fix:
Add a call to `refreshFiles()` in the `handleConfigUpdate()` function to immediately refresh the file list after updating the watch directory.

### Implementation:
In `frontend/src/App.tsx`, modify the `handleConfigUpdate()` function:

```typescript
const handleConfigUpdate = async () => {
  // Refresh the watch directory after config update
  try {
    const response = await fetch('/watch_directory')
    if (response.ok) {
      const data = await response.text()
      setWatchDirectory(data)
    }
  } catch (error) {
    console.error('Error fetching updated watch directory:', error)
  }
  
  // Refresh the file list to show files from the new directory
  await refreshFiles()
  
  // Close the config editor
  setIsConfigEditorOpen(false)
}
```

### Additional Considerations:

1. **Clear search state**: When the directory changes, any existing search results should be cleared since they're from the old directory:
   ```typescript
   // Clear search state when directory changes
   setSearchTerm('')
   setSearchLineNumbers({})
   setExpandedFiles(new Set())
   ```

2. **Clear transcript data**: Any loaded transcript data should be cleared since it's from files in the old directory:
   ```typescript
   // Clear transcript data from old directory
   setTranscriptData({})
   ```

3. **Error handling**: Consider adding error handling for the `refreshFiles()` call to inform the user if the new directory is inaccessible.

## Complete Solution

```typescript
const handleConfigUpdate = async () => {
  // Refresh the watch directory after config update
  try {
    const response = await fetch('/watch_directory')
    if (response.ok) {
      const data = await response.text()
      setWatchDirectory(data)
    }
  } catch (error) {
    console.error('Error fetching updated watch directory:', error)
  }
  
  // Clear search state and transcript data from old directory
  setSearchTerm('')
  setSearchLineNumbers({})
  setExpandedFiles(new Set())
  setTranscriptData({})
  
  // Refresh the file list to show files from the new directory
  try {
    await refreshFiles()
  } catch (error) {
    console.error('Error refreshing files from new directory:', error)
  }
  
  // Close the config editor
  setIsConfigEditorOpen(false)
}
```

## Testing Strategy

To verify the fix:
1. Start with a watch directory containing some MP4 files
2. Verify files are displayed in the UI
3. Change the watch directory to a different path with different MP4 files
4. Verify that the file list immediately updates to show files from the new directory
5. Verify that the watch directory path in the header updates correctly
6. Verify that any search results or expanded transcripts are cleared

## Impact Assessment

- **User Experience**: Eliminates confusion by immediately showing the correct files
- **Performance**: Minimal impact - adds one additional API call during configuration updates
- **Risk**: Low - the fix is isolated to the configuration update flow
- **Backwards Compatibility**: No breaking changes

This fix ensures that users see immediate feedback when changing the watch directory configuration, providing a much better user experience.