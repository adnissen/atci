import React from 'react';

interface TranscriptViewProps {
  visible?: boolean;
  name: string;
  className?: string;
  searchTerm?: string;
}

// Extend Window interface for our custom handlers
declare global {
  interface Window {
    handleTimestampHover?: (name: string, timestamp: string) => void;
    handleTimestampLeave?: () => void;
  }
}

const TranscriptView: React.FC<TranscriptViewProps> = ({
  visible = false,
  name,
  className = '',
  searchTerm = ''
}) => {
  if (!visible) {
    return null;
  }

  const [content, setContent] = React.useState<string>('');
  const [loading, setLoading] = React.useState<boolean>(false);
  const [error, setError] = React.useState<string | null>(null);
  const [hoveredTimestamp, setHoveredTimestamp] = React.useState<string | null>(null);
  const [thumbnailUrl, setThumbnailUrl] = React.useState<string | null>(null);

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

  // Handle timestamp link hover
  const handleTimestampHover = (name: string, timestamp: string) => {
    const seconds = timestampToSeconds(timestamp);
    setHoveredTimestamp(timestamp);
    setThumbnailUrl(`/frame/${encodeURIComponent(name)}/${seconds}`);
  };

  const handleTimestampLeave = () => {
    setHoveredTimestamp(null);
    setThumbnailUrl(null);
  };

  // Filter content to show only lines with search term and 1 line above
  const filterContentForSearch = (text: string, searchTerm: string): string => {
    if (!searchTerm.trim()) {
      return text;
    }

    const lines = text.split('\n');
    const filteredLines: string[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineLower = line.toLowerCase();
      
      // Check if current line contains search term
      if (lineLower.includes(searchTermLower)) {
        // Add previous line if it exists and we haven't already added it
        if (i > 0 && !filteredLines.includes(lines[i - 1])) {
          filteredLines.push(lines[i - 1]);
        }
        // Add current line
        filteredLines.push(line);
      }
    }

    return filteredLines.join('\n');
  };

  // Highlight search term in text
  const highlightSearchTerm = (text: string, searchTerm: string): string => {
    if (!searchTerm.trim()) {
      return text;
    }

    const regex = new RegExp(`(${searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark class="bg-yellow-200 px-1 rounded">$1</mark>');
  };

  // Process content to replace timestamps with clickable links
  const processContentWithTimestamps = (text: string): string => {
    // Regex to match timestamp format 00:00:00.000
    const timestampRegex = /(\d{2}:\d{2}:\d{2}\.\d{3})/g;
    
    return text.replace(timestampRegex, (match) => {
      const seconds = timestampToSeconds(match);
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" 
                class="text-blue-600 hover:text-blue-800 underline cursor-pointer timestamp-link" 
                data-timestamp="${match}"
                onmouseover="window.handleTimestampHover('${name}', '${match}')"
                onmouseout="window.handleTimestampLeave()">${match}</a>`;
    });
  };

  // Set up global handlers for the dynamically created links
  React.useEffect(() => {
    window.handleTimestampHover = handleTimestampHover;
    window.handleTimestampLeave = handleTimestampLeave;

    return () => {
      delete window.handleTimestampHover;
      delete window.handleTimestampLeave;
    };
  }, []);

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

  // Process content: filter for search, highlight search term, then add timestamp links
  const filteredContent = filterContentForSearch(content, searchTerm);
  const highlightedContent = highlightSearchTerm(filteredContent, searchTerm);
  const processedContent = processContentWithTimestamps(highlightedContent);

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
          <div className="text-gray-600 text-left relative">
            {searchTerm && filteredContent.trim() === '' && (
              <div className="text-gray-500 italic">
                No matches found for "{searchTerm}" in this transcript.
              </div>
            )}
            {filteredContent.trim() !== '' && (
              <pre 
                className="text-left whitespace-pre-wrap font-mono text-sm leading-relaxed max-w-none overflow-x-auto"
                dangerouslySetInnerHTML={{ __html: processedContent }}
              />
            )}
            
            {/* Thumbnail overlay */}
            {hoveredTimestamp && thumbnailUrl && (
              <div 
                className="fixed z-50 bg-white border border-gray-300 rounded-lg shadow-lg p-2"
                style={{
                  left: '50%',
                  top: '20px',
                  transform: 'translateX(-50%)',
                  maxWidth: '200px',
                  maxHeight: '150px'
                }}
              >
                <img 
                  src={thumbnailUrl} 
                  alt={`Frame at ${hoveredTimestamp}`}
                  className="w-full h-auto object-contain"
                  onError={(e) => {
                    e.currentTarget.style.display = 'none';
                  }}
                />
                <div className="text-xs text-gray-600 text-center mt-1">
                  {hoveredTimestamp}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default TranscriptView; 