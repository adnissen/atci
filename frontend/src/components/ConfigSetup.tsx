import React, { useState, useEffect } from 'react';
import { Trash2, Plus, Download, CheckCircle } from 'lucide-react';
import {
  Table,
  TableBody,
  TableCell,
  TableRow,
} from './ui/table';

interface ConfigSetupProps {
  onConfigComplete: () => void;
  isEditMode?: boolean;
}

interface ConfigData {
  watch_directories: string[];
  whispercli_path: string;
  model_path?: string;
  model_name?: string;
  ffmpeg_path: string;
  ffprobe_path: string;
}

interface Model {
  name: string;
  downloaded: boolean;
  path: string;
}

const ConfigSetup: React.FC<ConfigSetupProps> = ({ onConfigComplete, isEditMode = false }) => {
  const [config, setConfig] = useState<ConfigData>({
    watch_directories: [''],
    whispercli_path: '',
    model_path: '',
    ffmpeg_path: '',
    ffprobe_path: ''
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [models, setModels] = useState<Model[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [modelSelection, setModelSelection] = useState<'custom' | string>('custom');
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null);

  // Fetch existing configuration when in edit mode
  useEffect(() => {
    if (isEditMode) {
      fetchCurrentConfig();
    }
    fetchModels();
  }, [isEditMode]);

  const fetchModels = async () => {
    setIsLoadingModels(true);
    try {
      const response = await fetch('/api/models');
      if (response.ok) {
        const data = await response.json();
        setModels(data.models || []);
      }
    } catch (error) {
      console.error('Error fetching models:', error);
    } finally {
      setIsLoadingModels(false);
    }
  };

  const downloadModel = async (modelName: string) => {
    setDownloadingModel(modelName);
    try {
      const response = await fetch('/api/models/download', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ model_name: modelName })
      });
      
      if (response.ok) {
        // Refresh models list after download
        await fetchModels();
        // If this is the selected model, update the config
        if (modelSelection === modelName) {
          setConfig(prev => ({
            ...prev,
            model_name: modelName,
            model_path: ''
          }));
        }
      } else {
        const error = await response.json();
        setErrors([error.message || 'Failed to download model']);
      }
    } catch (error) {
      setErrors(['Network error: Failed to download model']);
    } finally {
      setDownloadingModel(null);
    }
  };

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
          // Handle the new watch_directories array format
          const configData = {
            watch_directories: data.config.watch_directories && data.config.watch_directories.length > 0 
              ? data.config.watch_directories 
              : [data.config.watch_directory || ''],
            whispercli_path: data.config.whispercli_path || '',
            model_path: data.config.model_path || '',
            model_name: data.config.model_name || '',
            ffmpeg_path: data.config.ffmpeg_path || '',
            ffprobe_path: data.config.ffprobe_path || ''
          };
          setConfig(configData);
          
          // Set model selection based on config
          if (configData.model_name) {
            setModelSelection(configData.model_name);
          } else if (configData.model_path) {
            setModelSelection('custom');
          }
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
    const hasValidDirectories = config.watch_directories.some(dir => dir.trim() !== '');
    const hasModel = (modelSelection === 'custom' && config.model_path?.trim() !== '') || 
                    (modelSelection !== 'custom' && models.find(m => m.name === modelSelection)?.downloaded);
    
    return hasValidDirectories && 
           config.whispercli_path.trim() !== '' && 
           config.ffmpeg_path.trim() !== '' &&
           config.ffprobe_path.trim() !== '' &&
           hasModel;
  };

  const handleInputChange = (field: keyof Omit<ConfigData, 'watch_directories'>, value: string) => {
    setConfig(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleDirectoryChange = (index: number, value: string) => {
    const newDirectories = [...config.watch_directories];
    newDirectories[index] = value;
    setConfig(prev => ({
      ...prev,
      watch_directories: newDirectories
    }));
  };

  const addDirectory = () => {
    setConfig(prev => ({
      ...prev,
      watch_directories: [...prev.watch_directories, '']
    }));
  };

  const removeDirectory = (index: number) => {
    if (config.watch_directories.length > 1) {
      const newDirectories = config.watch_directories.filter((_, i) => i !== index);
      setConfig(prev => ({
        ...prev,
        watch_directories: newDirectories
      }));
    }
  };

  const handleCancel = () => {
    onConfigComplete();
  };

  const handleModelSelectionChange = (value: string) => {
    setModelSelection(value);
    if (value === 'custom') {
      setConfig(prev => ({
        ...prev,
        model_name: undefined,
        model_path: prev.model_path || ''
      }));
    } else {
      const model = models.find(m => m.name === value);
      if (model) {
        setConfig(prev => ({
          ...prev,
          model_name: value,
          model_path: ''
        }));
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setErrors([]);
    setSuccessMessage('');

    // Filter out empty directories
    const filteredDirectories = config.watch_directories.filter(dir => dir.trim() !== '');
    
    if (filteredDirectories.length === 0) {
      setErrors(['At least one watch directory is required']);
      setIsSubmitting(false);
      return;
    }

    // Prepare submit data based on model selection
    const submitData: any = {
      watch_directories: filteredDirectories,
      whispercli_path: config.whispercli_path,
      ffmpeg_path: config.ffmpeg_path,
      ffprobe_path: config.ffprobe_path
    };

    if (modelSelection === 'custom') {
      submitData.model_path = config.model_path;
    } else {
      submitData.model_name = modelSelection;
    }

    try {
      const response = await fetch('/config', {
        method: 'POST',
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(submitData)
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
        <label className="block text-sm font-medium text-foreground mb-2">
          Watch Directories
        </label>
        <div className="space-y-2">
          <div className="border border-input rounded-md">
            <Table>
              <TableBody>
                {config.watch_directories.map((dir, index) => (
                  <TableRow key={index}>
                    <TableCell className="p-2">
                      <input
                        type="text"
                        value={dir}
                        onChange={(e) => handleDirectoryChange(index, e.target.value)}
                        placeholder="/path/to/your/videos"
                        className="w-full px-2 py-1 text-sm border-0 bg-transparent text-foreground focus:outline-none focus:ring-1 focus:ring-ring focus:ring-inset rounded"
                      />
                    </TableCell>
                    <TableCell className="p-2 w-10">
                      <button
                        type="button"
                        onClick={() => removeDirectory(index)}
                        disabled={config.watch_directories.length === 1}
                        className="p-1 text-destructive hover:bg-destructive/10 rounded disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                        title="Remove directory"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <button
            type="button"
            onClick={addDirectory}
            className="flex items-center gap-1 px-2 py-1 text-xs bg-secondary text-secondary-foreground rounded hover:bg-secondary/80"
          >
            <Plus className="h-3 w-3" />
            Add Directory
          </button>
        </div>
        <p className="text-xs text-muted-foreground mt-1">
          Directories to monitor for video files. At least one directory is required.
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
        <label className="block text-sm font-medium text-foreground mb-1">
          Model Selection
        </label>
        <select
          value={modelSelection}
          onChange={(e) => handleModelSelectionChange(e.target.value)}
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          disabled={isLoadingModels}
        >
          <option value="custom">Custom Model Path</option>
          {models.length > 0 && (
            <>
              <optgroup label="Downloaded Models">
                {models.filter(m => m.downloaded).map(model => (
                  <option key={model.name} value={model.name}>
                    {model.name} ✓
                  </option>
                ))}
              </optgroup>
              <optgroup label="Available Models">
                {models.filter(m => !m.downloaded).map(model => (
                  <option key={model.name} value={model.name}>
                    {model.name}
                  </option>
                ))}
              </optgroup>
            </>
          )}
        </select>
        
        {modelSelection === 'custom' ? (
          <>
            <input
              type="text"
              value={config.model_path || ''}
              onChange={(e) => handleInputChange('model_path', e.target.value)}
              placeholder="/path/to/your/whisper.cpp/model.bin"
              className="w-full mt-2 px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
              required={modelSelection === 'custom'}
            />
            <p className="text-xs text-muted-foreground mt-1">
              Path to the Whisper model file (.bin)
            </p>
          </>
        ) : modelSelection !== 'custom' && (
          <>
            {models.find(m => m.name === modelSelection)?.downloaded ? (
              <div className="mt-2 flex items-center gap-2 text-xs text-green-600">
                <CheckCircle className="h-4 w-4" />
                Model is downloaded and ready to use
              </div>
            ) : (
              <button
                type="button"
                onClick={() => downloadModel(modelSelection)}
                disabled={downloadingModel !== null}
                className="mt-2 flex items-center gap-2 px-3 py-1.5 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Download className="h-3 w-3" />
                {downloadingModel === modelSelection ? 'Downloading...' : 'Download Model'}
              </button>
            )}
          </>
        )}
      </div>

      <div>
        <label htmlFor="ffmpeg_path" className="block text-sm font-medium text-foreground mb-1">
          FFmpeg Path
        </label>
        <input
          type="text"
          id="ffmpeg_path"
          value={config.ffmpeg_path}
          onChange={(e) => handleInputChange('ffmpeg_path', e.target.value)}
          placeholder="/path/to/ffmpeg"
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          required
        />
        <p className="text-xs text-muted-foreground mt-1">
          Path to the ffmpeg executable (auto-detected if available in PATH)
        </p>
      </div>

      <div>
        <label htmlFor="ffprobe_path" className="block text-sm font-medium text-foreground mb-1">
          FFprobe Path
        </label>
        <input
          type="text"
          id="ffprobe_path"
          value={config.ffprobe_path}
          onChange={(e) => handleInputChange('ffprobe_path', e.target.value)}
          placeholder="/path/to/ffprobe"
          className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
          required
        />
        <p className="text-xs text-muted-foreground mt-1">
          Path to the ffprobe executable (auto-detected if available in PATH)
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
        <div className="bg-accent/10 border border-accent/20 rounded-md p-3">
          <div className="text-sm text-accent-foreground">
            {successMessage}
          </div>
        </div>
      )}

      <div className="flex space-x-3">
        <button
          type="submit"
          disabled={isSubmitting || !hasAllConfigValues()}
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
            Configuration will be saved as <code className="bg-muted px-1 py-0.5 rounded text-xs">.atconfig</code> in your home directory
          </p>
        </div>
      </div>
    </div>
  );
};

export default ConfigSetup;