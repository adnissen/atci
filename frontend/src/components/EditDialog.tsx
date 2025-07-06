import React from 'react';

interface EditDialogProps {
  isOpen: boolean;
  title: string;
  initialValue: string;
  onSave: (text: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
  placeholder?: string;
  className?: string;
  isLargeMode?: boolean;
  targetLineNumber?: number; // Line number to scroll to
}

const EditDialog: React.FC<EditDialogProps> = ({
  isOpen,
  title,
  initialValue,
  onSave,
  onCancel,
  isSubmitting,
  placeholder,
  className = '',
  isLargeMode = false,
  targetLineNumber
}) => {
  const [text, setText] = React.useState(initialValue);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Reset text when dialog opens/closes or initialValue changes
  React.useEffect(() => {
    setText(initialValue);
  }, [initialValue, isOpen]);

  // Auto-scroll to target line when dialog opens
  React.useEffect(() => {
    if (isOpen && targetLineNumber && textareaRef.current) {
      // Small delay to ensure the dialog has rendered
      setTimeout(() => {
        const textarea = textareaRef.current;
        if (!textarea) return;

        const lines = textarea.value.split('\n');
        if (targetLineNumber <= lines.length) {
          // Calculate the character position of the target line
          let characterPosition = 0;
          for (let i = 0; i < targetLineNumber - 1; i++) {
            characterPosition += lines[i].length + 1; // +1 for newline
          }

          // Set cursor position to the beginning of the target line
          textarea.setSelectionRange(characterPosition, characterPosition);
          textarea.focus();

          // Calculate more accurate line height
          const computedStyle = getComputedStyle(textarea);
          let lineHeight = parseInt(computedStyle.lineHeight);
          
          // If lineHeight is 'normal' or invalid, calculate it from font size
          if (isNaN(lineHeight) || lineHeight === 0) {
            const fontSize = parseInt(computedStyle.fontSize) || 14;
            lineHeight = Math.floor(fontSize * 1.4); // Typical line height multiplier
          }

          // Get textarea dimensions
          const textareaHeight = textarea.clientHeight;
          const paddingTop = parseInt(computedStyle.paddingTop) || 0;
          const paddingBottom = parseInt(computedStyle.paddingBottom) || 0;
          const usableHeight = textareaHeight - paddingTop - paddingBottom;

          // Calculate scroll position to center the target line
          const targetLineTop = (targetLineNumber - 1) * lineHeight;
          const centerOffset = usableHeight / 2;
          const scrollTop = Math.max(0, targetLineTop - centerOffset + paddingTop);

          textarea.scrollTop = scrollTop;
        }
      }, 100);
    }
  }, [isOpen, targetLineNumber]);

  const handleSave = () => {
    onSave(text);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className={`bg-white dark:bg-gray-800 p-6 rounded-lg ${isLargeMode ? 'max-w-4xl' : 'max-w-2xl'} w-full mx-4`}>
        <h3 className="text-lg font-semibold mb-4">{title}</h3>
        
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          className={`w-full ${isLargeMode ? 'h-96' : 'h-32'} p-3 border border-gray-300 dark:border-gray-600 rounded-md font-mono text-sm ${className}`}
          placeholder={placeholder}
        />
        
        <div className="flex justify-end gap-2 mt-4">
          <button
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 border border-gray-300 rounded-md hover:bg-gray-200 disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={isSubmitting}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {isSubmitting ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default EditDialog;