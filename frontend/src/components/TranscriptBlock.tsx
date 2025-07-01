import React from 'react';

interface TranscriptBlockProps {
  startTime?: string;
  endTime?: string;
  visible: boolean;
  text: string;
  name: string;
  isSearchResult?: boolean;
  lineNumbers: number[];
}

const TranscriptBlock: React.FC<TranscriptBlockProps> = ({
  startTime,
  endTime,
  visible,
  text,
  name,
  isSearchResult = false,
  lineNumbers
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
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" class="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 underline cursor-pointer timestamp-link" data-timestamp="${match}" onmouseover="window.handleTimestampHover('${name}', '${match}')" onmouseout="window.handleTimestampLeave()">${match}</a>`;
    });
  };

  // Process the text content (only timestamps, no icons)
  const processedText = processContentWithTimestamps(text);

  // Only return early if we have both start and end times and they're equal
  if (startTime && endTime && startTime === endTime) {
    return <></>;
  }

  // Determine which line number to show
  const timestampLineNumber = lineNumbers[0];
  const contentLineNumber = lineNumbers[lineNumbers.length - 1];

  return (
    <div className={`mb-2 ${isSearchResult ? 'bg-yellow-50 dark:bg-yellow-900/20 border-l-4 border-yellow-400 dark:border-yellow-600 pl-2' : ''}`}>
      <div className="grid grid-cols-12 gap-1">
        {startTime && endTime && (
          <div className="col-span-12 text-gray-500 dark:text-gray-400 text-sm font-mono flex items-center gap-2">
            <span className="text-gray-400 dark:text-gray-500 text-xs mr-2 flex-shrink-0 text-right w-8">{timestampLineNumber}</span>
            <span>
              <a 
                href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(startTime)}`}
                className="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 underline cursor-pointer"
                target="_blank"
              >
                {startTime}
              </a>
              {' --> '}
              <a 
                href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(endTime)}`}
                className="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 underline cursor-pointer"
                target="_blank"
              >
                {endTime}
              </a>
            </span>
            
            {/* Camera icon */}
            <span className="inline-flex items-center cursor-pointer text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors">
              <a href={`/frame/${encodeURIComponent(name)}/${timestampToSeconds(startTime) + (timestampToSeconds(endTime) - timestampToSeconds(startTime)) / 2}?text=${encodeURIComponent(text)}`} target="_blank" className="inline-flex items-baseline">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block">
                  <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/>
                  <circle cx="12" cy="13" r="3"/>
                </svg>
              </a>
            </span>
            
            {/* Video icon */}
            <span className="inline-flex items-center cursor-pointer text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors">
              <a href={`/clip?filename=${encodeURIComponent(name)}&start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}`} target="_blank" className="inline-flex items-baseline">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block">
                  <polygon points="23 7 16 12 23 17 23 7"/>
                  <rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
                </svg>
              </a>
            </span>
          </div>
        )}
        <div className="col-span-12 flex items-center -mt-1">
          <span className="text-gray-400 dark:text-gray-500 text-xs mr-2 flex-shrink-0 text-right w-8">{contentLineNumber}</span>
          <div className="w-4"></div>
          <div 
              className="text-gray-800 dark:text-gray-200 font-mono text-sm leading-relaxed whitespace-pre-wrap break-words w-full"
              dangerouslySetInnerHTML={{ __html: processedText }}
          />
        </div>
      </div>
    </div>
  );
};

export default TranscriptBlock; 