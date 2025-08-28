import React from 'react';
import { addTimestamp } from '../lib/utils';

interface DualEditDialogProps {
  isOpen: boolean;
  filename: string;
  transcriptInitialValue: string;
  onTranscriptSave: (text: string) => void;
  onCancel: () => void;
  isTranscriptSubmitting: boolean;
  transcriptTargetLineNumber?: number;
}

const DualEditDialog: React.FC<DualEditDialogProps> = ({
  isOpen,
  filename,
  transcriptInitialValue,
  onTranscriptSave,
  onCancel,
  isTranscriptSubmitting,
  transcriptTargetLineNumber
}) => {
  const [transcriptContent, setTranscriptContent] = React.useState(transcriptInitialValue);
  const transcriptTextareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Reset state when dialog opens/closes
  React.useEffect(() => {
    if (isOpen) {
      setTranscriptContent(transcriptInitialValue);
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


  const handleTranscriptSave = () => {
    onTranscriptSave(transcriptContent);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-card border border-border rounded-lg max-w-5xl w-full mx-4 max-h-[90vh] flex flex-col">
        {/* Header with close button */}
        <div className="flex justify-between items-center p-6 border-b border-border">
          <h3 className="text-lg font-semibold text-foreground">Edit Transcript - {filename}</h3>
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
        </div>
      </div>
    </div>
  );
};

export default DualEditDialog;