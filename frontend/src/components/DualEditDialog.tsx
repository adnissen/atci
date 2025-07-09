import React from 'react';

interface DualEditDialogProps {
  isOpen: boolean;
  filename: string;
  transcriptInitialValue: string;
  metaInitialValue: string;
  onTranscriptSave: (text: string) => void;
  onMetaSave: (text: string) => void;
  onCancel: () => void;
  isTranscriptSubmitting: boolean;
  transcriptTargetLineNumber?: number;
}

const DualEditDialog: React.FC<DualEditDialogProps> = ({
  isOpen,
  filename,
  transcriptInitialValue,
  metaInitialValue,
  onTranscriptSave,
  onMetaSave,
  onCancel,
  isTranscriptSubmitting,
  transcriptTargetLineNumber
}) => {
  const [metaContent, setMetaContent] = React.useState(metaInitialValue);
  const [transcriptContent, setTranscriptContent] = React.useState(transcriptInitialValue);
  const [isLoadingMeta, setIsLoadingMeta] = React.useState(false);
  const [isSavingMeta, setIsSavingMeta] = React.useState(false);
  const transcriptTextareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Reset state when dialog opens/closes
  React.useEffect(() => {
    if (isOpen) {
      setTranscriptContent(transcriptInitialValue);
      fetchMetaContent();
    }
  }, [isOpen, transcriptInitialValue]);

  // Auto-scroll to target line when dialog opens
  React.useEffect(() => {
    if (isOpen && transcriptTargetLineNumber && transcriptTextareaRef.current) {
      // Small delay to ensure the dialog has rendered
      setTimeout(() => {
        const textarea = transcriptTextareaRef.current;
        if (!textarea) return;

        const lines = textarea.value.split('\n');
        if (transcriptTargetLineNumber <= lines.length) {
          // Calculate the character position of the target line
          let characterPosition = 0;
          for (let i = 0; i < transcriptTargetLineNumber - 1; i++) {
            characterPosition += lines[i].length + 1; // +1 for newline
          }

          // Set cursor position to the beginning of the target line
          textarea.setSelectionRange(characterPosition, characterPosition);
          textarea.focus();

          // Scroll to the cursor position
          // Calculate approximate line height and scroll position
          const lineHeight = textarea.scrollHeight / lines.length;
          const approximateScrollTop = (transcriptTargetLineNumber - 1) * lineHeight;
          // Move target line to the middle by adding half the height of the textarea
          textarea.scrollTop = approximateScrollTop - (textarea.clientHeight / 2);
        }
      }, 100);
    }
  }, [isOpen, transcriptTargetLineNumber]);

  const fetchMetaContent = async () => {
    if (!filename) return;
    
    setIsLoadingMeta(true);
    try {
      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/meta`);
      if (response.ok) {
        const data = await response.json();
        setMetaContent(data.content || '');
      } else {
        console.error('Failed to fetch meta content');
        setMetaContent('');
      }
    } catch (error) {
      console.error('Error fetching meta content:', error);
      setMetaContent('');
    } finally {
      setIsLoadingMeta(false);
    }
  };

  const handleMetaSave = async () => {
    setIsSavingMeta(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/meta`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ content: metaContent }),
      });

      if (response.ok) {
        onMetaSave(metaContent);
      } else {
        const error = await response.json();
        alert(`Error: ${error.error || 'Failed to update meta file'}`);
      }
    } catch (error) {
      console.error('Error updating meta file:', error);
      alert('Error: Failed to update meta file. Please try again.');
    } finally {
      setIsSavingMeta(false);
    }
  };

  const handleTranscriptSave = () => {
    onTranscriptSave(transcriptContent);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-card border border-border rounded-lg max-w-5xl w-full mx-4 max-h-[90vh] flex flex-col">
        {/* Header with close button */}
        <div className="flex justify-between items-center p-6 border-b border-border">
          <h3 className="text-lg font-semibold text-foreground">Edit Files - {filename}</h3>
          <button
            onClick={onCancel}
            className="p-1 text-muted-foreground hover:text-foreground hover:bg-accent rounded transition-colors"
            title="Close"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content area */}
        <div className="flex-1 p-6 overflow-y-auto">
          {/* Transcript Edit Section */}
          <div className="mb-6">
            <div className="flex justify-between items-center mb-2">
              <h4 className="text-md font-medium text-foreground">Transcript (.txt)</h4>
              <button
                onClick={handleTranscriptSave}
                disabled={isTranscriptSubmitting}
                className="px-3 py-1.5 text-xs font-normal text-primary-foreground bg-primary border border-transparent rounded hover:bg-primary/90 disabled:opacity-50 transition-colors"
              >
                {isTranscriptSubmitting ? 'Saving...' : 'Save Transcript'}
              </button>
            </div>
            <textarea
              ref={transcriptTextareaRef}
              value={transcriptContent}
              onChange={(e) => setTranscriptContent(e.target.value)}
              className="w-full h-64 p-3 border border-input bg-background text-foreground rounded-md font-mono text-sm leading-6 focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
              placeholder="Enter transcript content..."
              readOnly={isTranscriptSubmitting}
            />
          </div>
          
          {/* Meta File Edit Section */}
          <div className="mb-4">
            <div className="flex justify-between items-center mb-2">
              <h4 className="text-md font-medium text-foreground">Meta File (.meta)</h4>
              <button
                onClick={handleMetaSave}
                disabled={isSavingMeta || isLoadingMeta}
                className="px-3 py-1.5 text-xs font-normal text-primary-foreground bg-primary border border-transparent rounded hover:bg-primary/90 disabled:opacity-50 transition-colors"
              >
                {isSavingMeta ? 'Saving...' : 'Save Meta'}
              </button>
            </div>
            {isLoadingMeta ? (
              <div className="w-full h-32 p-3 border border-input bg-background rounded-md flex items-center justify-center">
                <span className="text-muted-foreground">Loading meta file...</span>
              </div>
            ) : (
              <textarea
                value={metaContent}
                onChange={(e) => setMetaContent(e.target.value)}
                className="w-full h-32 p-3 border border-input bg-background text-foreground rounded-md font-mono text-sm leading-6 focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
                placeholder="Enter meta file content (key: value format)..."
                readOnly={isSavingMeta}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default DualEditDialog;