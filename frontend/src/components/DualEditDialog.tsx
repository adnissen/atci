import React from 'react';
import EditDialog from './EditDialog';

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
  const [isLoadingMeta, setIsLoadingMeta] = React.useState(false);
  const [isSavingMeta, setIsSavingMeta] = React.useState(false);
  const [transcriptText, setTranscriptText] = React.useState(transcriptInitialValue);
  const [showTranscriptDialog, setShowTranscriptDialog] = React.useState(false);
  const [showMetaDialog, setShowMetaDialog] = React.useState(false);

  // Reset state when dialog opens/closes
  React.useEffect(() => {
    if (isOpen) {
      setTranscriptText(transcriptInitialValue);
      fetchMetaContent();
    } else {
      setShowTranscriptDialog(false);
      setShowMetaDialog(false);
    }
  }, [isOpen, transcriptInitialValue]);

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

  const handleMetaSave = async (text: string) => {
    setIsSavingMeta(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/meta`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ content: text }),
      });

      if (response.ok) {
        setMetaContent(text);
        onMetaSave(text);
        setShowMetaDialog(false);
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

  const handleTranscriptSave = (text: string) => {
    onTranscriptSave(text);
    setShowTranscriptDialog(false);
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Main Dialog */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm flex items-center justify-center z-50">
        <div className="bg-card border border-border p-6 rounded-lg max-w-5xl w-full mx-4">
          <h3 className="text-lg font-semibold mb-4 text-foreground">Edit Files - {filename}</h3>
          
          {/* Transcript Preview Section */}
          <div className="mb-6">
            <div className="flex justify-between items-center mb-2">
              <h4 className="text-md font-medium text-foreground">Transcript (.txt)</h4>
              <button
                onClick={() => setShowTranscriptDialog(true)}
                className="px-3 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90"
              >
                Edit Transcript
              </button>
            </div>
            <div className="w-full h-64 p-3 border border-input bg-background text-foreground rounded-md font-mono text-sm leading-6 overflow-y-auto">
              <pre className="whitespace-pre-wrap">{transcriptText || 'No transcript content'}</pre>
            </div>
          </div>
          
          {/* Meta File Preview Section */}
          <div className="mb-4">
            <div className="flex justify-between items-center mb-2">
              <h4 className="text-md font-medium text-foreground">Meta File (.meta)</h4>
              <button
                onClick={() => setShowMetaDialog(true)}
                disabled={isLoadingMeta}
                className="px-3 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50"
              >
                Edit Meta
              </button>
            </div>
            {isLoadingMeta ? (
              <div className="w-full h-32 p-3 border border-input bg-background rounded-md flex items-center justify-center">
                <span className="text-muted-foreground">Loading meta file...</span>
              </div>
            ) : (
              <div className="w-full h-32 p-3 border border-input bg-background text-foreground rounded-md font-mono text-sm leading-6 overflow-y-auto">
                <pre className="whitespace-pre-wrap">{metaContent || 'No meta content'}</pre>
              </div>
            )}
          </div>
          
          {/* Close Button */}
          <div className="flex justify-end">
            <button
              onClick={onCancel}
              className="px-4 py-2 text-sm font-medium text-secondary-foreground bg-secondary border border-input rounded-md hover:bg-secondary/80 transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>

      {/* Transcript Edit Dialog */}
      <EditDialog
        isOpen={showTranscriptDialog}
        title={`Edit Transcript - ${filename}`}
        initialValue={transcriptText}
        onSave={handleTranscriptSave}
        onCancel={() => setShowTranscriptDialog(false)}
        isSubmitting={isTranscriptSubmitting}
        placeholder="Enter transcript content..."
        isLargeMode={true}
        targetLineNumber={transcriptTargetLineNumber}
      />

      {/* Meta File Edit Dialog */}
      <EditDialog
        isOpen={showMetaDialog}
        title={`Edit Meta File - ${filename}`}
        initialValue={metaContent}
        onSave={handleMetaSave}
        onCancel={() => setShowMetaDialog(false)}
        isSubmitting={isSavingMeta}
        placeholder="Enter meta file content (key: value format)..."
        className="h-48"
      />
    </>
  );
};

export default DualEditDialog;