import React from 'react';

interface TranscriptViewProps {
  visible?: boolean;
  name: string;
  className?: string;
}

const TranscriptView: React.FC<TranscriptViewProps> = ({
  visible = false,
  name,
  className = ''
}) => {
  if (!visible) {
    return null;
  }

  const [content, setContent] = React.useState<string>('');
  const [loading, setLoading] = React.useState<boolean>(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    const fetchTranscript = async () => {
      setLoading(true);
      setError(null);
      
      try {
        const response = await fetch(`/transcripts/${encodeURIComponent(name)}`);
        
        if (!response.ok) {
          throw new Error(`Failed to fetch transcript: ${response.status} ${response.statusText}`);
        }
        
        const transcriptContent = await response.text();
        setContent(transcriptContent);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An unknown error occurred');
      } finally {
        setLoading(false);
      }
    };

    if (visible && name) {
      fetchTranscript();
    }
  }, [visible, name]);
  return (
    <div className={`w-full p-6 bg-white ${className}`}>
      <div className="space-y-4">
        {/* Transcript content will go here */}
        <div className="text-gray-600 text-left">
          <pre className="text-left">{content}</pre>
        </div>
      </div>
    </div>
  );
};

export default TranscriptView; 