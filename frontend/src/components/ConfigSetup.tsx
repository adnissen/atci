import React, { useState, useEffect } from 'react';

interface ConfigSetupProps {
  onConfigComplete: () => void;
  isEditMode?: boolean;
}

interface ConfigData {
  watch_directory: string;
  whispercli_path: string;
  model_path: string;
}

const ConfigSetup: React.FC<ConfigSetupProps> = ({ onConfigComplete, isEditMode = false }) => {
  const [config, setConfig] = useState<ConfigData>({
    watch_directory: '',
    whispercli_path: '',
    model_path: ''
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);

  // Fetch existing configuration when in edit mode
  useEffect(() => {
    if (isEditMode) {
      fetchCurrentConfig();
    }
  }, [isEditMode]);

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

  // Check if all config values are present (not empty)
  const hasAllConfigValues = () => {
    return config.watch_directory.trim() !== '' && 
           config.whispercli_path.trim() !== '' && 
           config.model_path.trim() !== '';
  };

  const handleInputChange = (field: keyof ConfigData, value: string) => {
    setConfig(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleCancel = () => {
    onConfigComplete();
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
        const message = isEditMode ? 'Configuration updated successfully!' : 'Configuration saved successfully!';
        setSuccessMessage(message);
        setTimeout(() => {
          onConfigComplete();
        }, 1500);
      } else {
        const errorMessage = isEditMode ? 'Failed to update configuration' : 'Failed to save configuration';
        setErrors(result.errors || [result.message || errorMessage]);
      }
    } catch {
      const errorMessage = isEditMode ? 'Network error: Failed to update configuration' : 'Network error: Failed to save configuration';
      setErrors([errorMessage]);
    } finally {
      setIsSubmitting(false);
    }
  };

  const renderForm = () => (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label htmlFor="watch_directory" className="block text-sm font-medium text-foreground mb-1">
          Watch Directory
        </label>
        <input
          type="text"
          id="watch_directory"
          value={config.watch_directory}
          onChange={(e) => handleInputChange('watch_directory', e.target.value)}
          placeholder="/path/to/your/videos"
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          required
        />
        <p className="text-xs text-muted-foreground mt-1">
          Directory to monitor for MP4 video files
        </p>
      </div>

      <div>
        <label htmlFor="whispercli_path" className="block text-sm font-medium text-foreground mb-1">
          Whisper CLI Path
        </label>
        <input
          type="text"
          id="whispercli_path"
          value={config.whispercli_path}
          onChange={(e) => handleInputChange('whispercli_path', e.target.value)}
          placeholder="/path/to/whisper-cli"
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          required
        />
        <p className="text-xs text-muted-foreground mt-1">
          Path to the whisper.cpp CLI executable
        </p>
      </div>

      <div>
        <label htmlFor="model_path" className="block text-sm font-medium text-foreground mb-1">
          Model Path
        </label>
        <input
          type="text"
          id="model_path"
          value={config.model_path}
          onChange={(e) => handleInputChange('model_path', e.target.value)}
          placeholder="/path/to/your/whisper.cpp/model.bin"
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          required
        />
        <p className="text-xs text-muted-foreground mt-1">
          Path to the Whisper model file (.bin)
        </p>
      </div>

      {errors.length > 0 && (
        <div className="bg-destructive/10 border border-destructive/20 rounded-md p-3">
          <div className="text-sm text-destructive">
            <ul className="list-disc list-inside space-y-1">
              {errors.map((error, index) => (
                <li key={index}>{error}</li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {successMessage && (
        <div className="bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-800 rounded-md p-3">
          <div className="text-sm text-green-800 dark:text-green-200">
            {successMessage}
          </div>
        </div>
      )}

      <div className="flex space-x-3">
        <button
          type="submit"
          disabled={isSubmitting}
          className="flex-1 bg-primary text-primary-foreground py-2 px-4 rounded-md hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? 'Saving...' : (isEditMode ? 'Update Configuration' : 'Save Configuration')}
        </button>
        
        {hasAllConfigValues() && (
          <button
            type="button"
            onClick={handleCancel}
            disabled={isSubmitting}
            className="flex-1 bg-secondary text-secondary-foreground py-2 px-4 rounded-md hover:bg-secondary/80 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancel
          </button>
        )}
      </div>
    </form>
  );

  // Always render in full-screen mode
  return (
    <div className="min-h-screen bg-background flex items-center justify-center px-4">
      <div className="max-w-md w-full bg-card border border-border rounded-lg shadow-md p-6">
        <div className="text-center mb-6">
          <h1 className="text-2xl font-bold text-foreground mb-2">
            {isEditMode ? 'Edit Configuration' : 'Autotranscript Setup'}
          </h1>
          <p className="text-muted-foreground">
            {isEditMode ? 'Update your configuration settings' : 'Configure the required paths to get started'}
          </p>
        </div>

        {isLoading ? (
          <div className="text-center py-8">
            <div className="text-muted-foreground">Loading configuration...</div>
          </div>
        ) : (
          renderForm()
        )}

        <div className="mt-6 text-center">
          <p className="text-xs text-muted-foreground">
            Configuration will be saved as <code className="bg-muted px-1 py-0.5 rounded text-xs">.atconfig</code> in the current directory
          </p>
        </div>
      </div>
    </div>
  );
};

export default ConfigSetup;