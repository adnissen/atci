import React from 'react';

interface FileRowProps {
  filename: string;
  date: string;
  lineCount: number;
  length: string;
  className?: string;
}

const FileRow: React.FC<FileRowProps> = ({
  filename,
  date,
  lineCount,
  length,
  className = ''
}) => {
  return (
    <div className={`flex items-center justify-between p-4 border-b border-gray-200 hover:bg-gray-50 transition-colors ${className}`}>
      <div className="flex-1">
        <div className="font-medium text-gray-900">{filename}</div>
      </div>
      <div className="flex items-center space-x-8 text-sm text-gray-600">
        <div className="w-24 text-center">{date}</div>
        <div className="w-16 text-center">{lineCount}</div>
        <div className="w-20 text-center">{length}</div>
      </div>
    </div>
  );
};

export default FileRow; 