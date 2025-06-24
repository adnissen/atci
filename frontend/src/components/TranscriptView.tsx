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

  // Convert timestamp format 00:00:00.000 to seconds
  const timestampToSeconds = (timestamp: string): number => {
    const parts = timestamp.split(':');
    const hours = parseInt(parts[0], 10);
    const minutes = parseInt(parts[1], 10);
    const secondsParts = parts[2].split('.');
    const seconds = parseInt(secondsParts[0], 10);
    const milliseconds = parseInt(secondsParts[1] || '0', 10);
    
    return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000;
  };

  // Process content to replace timestamps with clickable links
  const processContentWithTimestamps = (text: string): string => {
    // Regex to match timestamp format 00:00:00.000
    const timestampRegex = /(\d{2}:\d{2}:\d{2}\.\d{3})/g;
    
    return text.replace(timestampRegex, (match) => {
      const seconds = timestampToSeconds(match);
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" class="text-blue-600 hover:text-blue-800 underline cursor-pointer">${match}</a>`;
    });
  };

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

  // Process content to add timestamp links
  const processedContent = processContentWithTimestamps(content);

  return (
    <div className={`w-full p-6 bg-white ${className}`}>
      <div className="space-y-4">
        {loading && (
          <div className="text-gray-600">Loading transcript...</div>
        )}
        {error && (
          <div className="text-red-600">Error: {error}</div>
        )}
        {!loading && !error && (
          <div className="text-gray-600 text-left">
            <pre 
              className="text-left whitespace-pre-wrap font-mono text-sm leading-relaxed max-w-none overflow-x-auto"
              dangerouslySetInnerHTML={{ __html: processedContent }}
            />
          </div>
        )}
      </div>
    </div>
  );
};

export default TranscriptView; 