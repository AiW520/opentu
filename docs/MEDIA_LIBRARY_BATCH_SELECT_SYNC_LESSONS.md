# Media Library Batch Select Sync Lessons

## Problem
The media library buttons in two different locations (`AIInputBar` and `EnhancedChatInput`) had inconsistent behavior:
- The media library in the AI input bar had full batch select functionality
- The media library in the chat drawer only had single select and lacked the batch select "use" button

## Root Cause
The `MediaLibraryModal` component requires both `onSelect` and `onSelectMultiple` props to enable full functionality. The `EnhancedChatInput` was only passing `onSelect` but missing `onSelectMultiple`.

## Solution
Add `handleMediaLibrarySelectMultiple` function to `EnhancedChatInput` and pass it to `MediaLibraryModal`.

## Code Changes

### File: `packages/drawnix/src/components/chat-drawer/EnhancedChatInput.tsx`

#### 1. Added `handleMediaLibrarySelectMultiple` function
```tsx
const handleMediaLibrarySelectMultiple = useCallback(
  async (assets: Asset[]) => {
    if (assets.length === 0) return;

    try {
      const newContents = assets.map((asset) => ({
        type: 'image' as const,
        url: asset.url,
        name: asset.name || `素材-${Date.now()}`,
      }));
      appendUploadedContent(newContents);
      setShowMediaLibrary(false);
    } catch (error) {
      console.error('Failed to batch select assets from library:', error);
      setShowMediaLibrary(false);
    }
  },
  [appendUploadedContent]
);
```

#### 2. Updated `MediaLibraryModal` call
```tsx
<MediaLibraryModal
  isOpen={showMediaLibrary}
  onClose={() => setShowMediaLibrary(false)}
  mode={SelectionMode.SELECT}
  filterType={AssetType.IMAGE}
  onSelect={handleMediaLibrarySelect}
  onSelectMultiple={handleMediaLibrarySelectMultiple}  // New
  batchSelectButtonText="批量插入对话框"              // New
/>
```

## Architecture Notes

### Component Hierarchy
```
AssetContext (Shared state)
├── MediaLibraryModal (Reusable component)
│   ├── MediaLibraryGrid
│   └── MediaLibraryInspector
├── AIInputBar (Has full functionality)
└── EnhancedChatInput (Now also has full functionality)
```

### Key Patterns
1. **Context-based state management** - `AssetContext` provides shared media data
2. **Reusable component design** - `MediaLibraryModal` is used across different parts of the app
3. **Progressive enhancement** - Features are enabled based on passed props
4. **Consistent UX** - Both input areas now have identical media library behavior

## Lessons Learned

1. **Check for prop consistency** - When reusing components across different locations, ensure all required props are passed
2. **Maintain UX parity** - Similar features should behave the same way regardless of where they appear
3. **Design for extensibility** - Reusable components should accept optional props for different use cases
4. **Batch operations add value** - Users appreciate the ability to select and insert multiple items at once

## Verification
Both media library buttons now have:
- ✅ Batch select button available
- ✅ Batch select functionality works
- ✅ "Use" button enabled and functional
- ✅ Consistent UI behavior
