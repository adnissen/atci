import React, { useState, useEffect } from 'react';

interface ConfigEditorProps {
  isOpen: boolean;
  onClose: () => void;
  onConfigUpdate: () => void;
}

interface ConfigData {
  watch_directory: string;
  whispercli_path: string;
  model_path: string;
}

const ConfigEditor: React.FC<ConfigEditorProps> = ({ isOpen, onClose, onConfigUpdate }) => {
  const [config, setConfig] = useState<ConfigData>({
    watch_directory: '',
    whispercli_path: '',
    model_path: ''
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);

  // Fetch existing configuration when modal opens
  useEffect(() => {
    if (isOpen) {
      fetchCurrentConfig();
    }
  }, [isOpen]);

  const fetchCurrentConfig = async () => {
    setIsLoading(true);
    try {
      const response = await fetch('/config', {
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json'
        }
      });
      if (response.ok) {
        const data = await response.json();
        if (data.config) {
          setConfig(data.config);
        }
      }
    } catch (error) {
      console.error('Error fetching configuration:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleInputChange = (field: keyof ConfigData, value: string) => {
    setConfig(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setErrors([]);
    setSuccessMessage('');

    try {
      const response = await fetch('/config', {
        method: 'POST',
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(config)
      });

      const result = await response.json();

      if (response.ok && result.success) {
        setSuccessMessage('Configuration updated successfully!');
        setTimeout(() => {
          onConfigUpdate();
          onClose();
        }, 1500);
      } else {
        setErrors(result.errors || [result.message || 'Failed to update configuration']);
      }
    } catch {
      setErrors(['Network error: Failed to update configuration']);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    setErrors([]);
    setSuccessMessage('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center px-4 z-50">
      <div className="max-w-md w-full bg-white rounded-lg shadow-lg p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-bold text-gray-900">
            Edit Configuration
          </h2>
          <button
            onClick={handleClose}
            className="text-gray-400 hover:text-gray-600 transition-colors"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {isLoading ? (
          <div className="text-center py-8">
            <div className="text-gray-600">Loading configuration...</div>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="watch_directory" className="block text-sm font-medium text-gray-700 mb-1">
                Watch Directory
              </label>
              <input
                type="text"
                id="watch_directory"
                value={config.watch_directory}
                onChange={(e) => handleInputChange('watch_directory', e.target.value)}
                placeholder="/path/to/your/videos"
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                required
              />
              <p className="text-xs text-gray-500 mt-1">
                Directory to monitor for MP4 video files
              </p>
            </div>

            <div>
              <label htmlFor="whispercli_path" className="block text-sm font-medium text-gray-700 mb-1">
                Whisper CLI Path
              </label>
              <input
                type="text"
                id="whispercli_path"
                value={config.whispercli_path}
                onChange={(e) => handleInputChange('whispercli_path', e.target.value)}
                placeholder="/path/to/whisper-cli"
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                required
              />
              <p className="text-xs text-gray-500 mt-1">
                Path to the whisper.cpp CLI executable
              </p>
            </div>

            <div>
              <label htmlFor="model_path" className="block text-sm font-medium text-gray-700 mb-1">
                Model Path
              </label>
              <input
                type="text"
                id="model_path"
                value={config.model_path}
                onChange={(e) => handleInputChange('model_path', e.target.value)}
                placeholder="/path/to/your/whisper.cpp/model.bin"
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                required
              />
              <p className="text-xs text-gray-500 mt-1">
                Path to the Whisper model file (.bin)
              </p>
            </div>

            {errors.length > 0 && (
              <div className="bg-red-50 border border-red-200 rounded-md p-3">
                <div className="text-sm text-red-800">
                  <ul className="list-disc list-inside space-y-1">
                    {errors.map((error, index) => (
                      <li key={index}>{error}</li>
                    ))}
                  </ul>
                </div>
              </div>
            )}

            {successMessage && (
              <div className="bg-green-50 border border-green-200 rounded-md p-3">
                <div className="text-sm text-green-800">
                  {successMessage}
                </div>
              </div>
            )}

            <div className="flex gap-3 pt-4">
              <button
                type="button"
                onClick={handleClose}
                className="flex-1 bg-gray-200 text-gray-800 py-2 px-4 rounded-md hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting ? 'Saving...' : 'Save Changes'}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
};

export default ConfigEditor;