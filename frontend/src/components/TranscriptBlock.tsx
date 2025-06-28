import React from 'react';

interface TranscriptBlockProps {
  startTime?: string;
  endTime?: string;
  visible: boolean;
  text: string;
  name: string;
}

const TranscriptBlock: React.FC<TranscriptBlockProps> = ({
  startTime,
  endTime,
  visible,
  text,
  name
}) => {
  if (!visible || text === "WEBVTT") {
    return null;
  }

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
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" class="text-blue-600 hover:text-blue-800 underline cursor-pointer timestamp-link" data-timestamp="${match}" onmouseover="window.handleTimestampHover('${name}', '${match}')" onmouseout="window.handleTimestampLeave()">${match}</a>`;
    });
  };

  // Process the text content (only timestamps, no icons)
  const processedText = processContentWithTimestamps(text);

  return (
    <div className="mb-2">
      {startTime && endTime && (
        <div className="text-gray-500 text-sm font-mono flex items-center gap-2">
          <span>
            <a 
              href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(startTime)}`}
              className="text-blue-600 hover:text-blue-800 underline cursor-pointer"
              target="_blank"
            >
              {startTime}
            </a>
            {' --> '}
            <a 
              href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(endTime)}`}
              className="text-blue-600 hover:text-blue-800 underline cursor-pointer"
              target="_blank"
            >
              {endTime}
            </a>
          </span>
          
          {/* Camera icon */}
          <span className="inline-flex items-center cursor-pointer text-gray-600 hover:text-blue-600 transition-colors">
            <a href={`/frame/${encodeURIComponent(name)}/${timestampToSeconds(startTime) + (timestampToSeconds(endTime) - timestampToSeconds(startTime)) / 2}?text=${encodeURIComponent(text)}`} target="_blank" className="inline-flex items-baseline">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block">
                <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/>
                <circle cx="12" cy="13" r="3"/>
              </svg>
            </a>
          </span>
          
          {/* Video icon */}
          <span className="inline-flex items-center cursor-pointer text-gray-600 hover:text-blue-600 transition-colors">
            <a href={`/clip?filename=${encodeURIComponent(name)}&start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}`} target="_blank" className="inline-flex items-baseline">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block">
                <polygon points="23 7 16 12 23 17 23 7"/>
                <rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
              </svg>
            </a>
          </span>
        </div>
      )}
      <div 
        className="text-gray-800 font-mono text-sm leading-relaxed"
        dangerouslySetInnerHTML={{ __html: processedText }}
      />
    </div>
  );
};

export default TranscriptBlock; 