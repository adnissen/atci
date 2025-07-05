import React from 'react';

interface EditDialogProps {
  isOpen: boolean;
  title: string;
  value: string;
  onValueChange: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
  isSubmitting: boolean;
  placeholder?: string;
  className?: string;
}

const EditDialog: React.FC<EditDialogProps> = ({
  isOpen,
  title,
  value,
  onValueChange,
  onSave,
  onCancel,
  isSubmitting,
  placeholder,
  className = ''
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg max-w-2xl w-full mx-4">
        <h3 className="text-lg font-semibold mb-4">{title}</h3>
        
        <textarea
          value={value}
          onChange={(e) => onValueChange(e.target.value)}
          className={`w-full h-32 p-3 border border-gray-300 dark:border-gray-600 rounded-md font-mono text-sm ${className}`}
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
            onClick={onSave}
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