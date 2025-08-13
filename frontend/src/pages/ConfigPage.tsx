import { useState, useEffect } from 'react';
import { Trash2, Plus, Download, CheckCircle, ChevronLeft, AlertCircle } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import {
  Table,
  TableBody,
  TableCell,
  TableRow,
} from '../components/ui/table';
import { addTimestamp } from '../lib/utils';

interface ConfigData {
  watch_directories: string[];
  whispercli_path: string;
  model_path?: string;
  model_name?: string;
  ffmpeg_path: string;
  ffprobe_path: string;
  nonlocal_password?: string;
}

interface Model {
  name: string;
  downloaded: boolean;
  path: string;
}

interface FFmpegTool {
  name: string;
  platform: string;
  downloaded: boolean;
  downloaded_path: string;
  system_available: boolean;
  system_path: string | null;
  current_path: string;
}

interface WhisperCliTool {
  name: string;
  platform: string;
  downloaded: boolean;
  downloaded_path: string;
  system_available: boolean;
  system_path: string | null;
  current_path: string;
}

interface ConfigPageProps {
  onClose?: () => void;
}

export default function ConfigPage({ onClose }: ConfigPageProps = {}) {
  const navigate = useNavigate();
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
  const [ffmpegTools, setFfmpegTools] = useState<FFmpegTool[]>([]);
  const [downloadingTool, setDownloadingTool] = useState<string | null>(null);
  const [whisperCliTools, setWhisperCliTools] = useState<WhisperCliTool[]>([]);
  const [downloadingWhisperTool, setDownloadingWhisperTool] = useState<string | null>(null);

  // Fetch existing configuration on mount
  useEffect(() => {
    fetchCurrentConfig();
    fetchModels();
    fetchFFmpegTools();
    fetchWhisperCliTools();
  }, []);

  const fetchModels = async () => {
    setIsLoadingModels(true);
    try {
      const response = await fetch(addTimestamp('/api/models'));
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
      const response = await fetch(addTimestamp('/api/models/download'), {
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

  const fetchFFmpegTools = async () => {
    try {
      const response = await fetch(addTimestamp('/api/ffmpeg/tools'));
      if (response.ok) {
        const data = await response.json();
        setFfmpegTools(data.tools);
      } else {
        console.error('Failed to fetch FFmpeg tools');
      }
    } catch (error) {
      console.error('Error fetching FFmpeg tools:', error);
    }
  };

  const fetchWhisperCliTools = async () => {
    try {
      const response = await fetch(addTimestamp('/api/whisper-cli/tools'));
      if (response.ok) {
        const data = await response.json();
        setWhisperCliTools(data.tools);
      } else {
        console.error('Failed to fetch Whisper-CLI tools');
      }
    } catch (error) {
      console.error('Error fetching Whisper-CLI tools:', error);
    }
  };

  const downloadFFmpegTool = async (toolName: string) => {
    setDownloadingTool(toolName);
    try {
      const response = await fetch(addTimestamp('/api/ffmpeg/download'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ tool_name: toolName }),
      });

      if (response.ok) {
        // Refresh tools list after download
        await fetchFFmpegTools();
        await fetchCurrentConfig();
        setSuccessMessage(`${toolName} downloaded successfully`);
      } else {
        const error = await response.json();
        setErrors([error.message || `Failed to download ${toolName}`]);
      }
    } catch (error: any) {
      console.error('Error downloading tool:', error);
      setErrors([error.message || `Failed to download ${toolName}`]);
    } finally {
      setDownloadingTool(null);
    }
  };

  const useDownloadedFFmpegTool = async (toolName: string) => {
    try {
      const response = await fetch(addTimestamp('/api/ffmpeg/use-downloaded'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ tool_name: toolName }),
      });

      if (response.ok) {
        await fetchCurrentConfig();
        await fetchFFmpegTools();
        setSuccessMessage(`Now using downloaded ${toolName}`);
      } else {
        const error = await response.json();
        setErrors([error.message || `Failed to use downloaded ${toolName}`]);
      }
    } catch (error: any) {
      console.error('Error setting tool path:', error);
      setErrors([error.message || `Failed to use downloaded ${toolName}`]);
    }
  };

  const useAutoDetectionForTool = async (toolName: string) => {
    const tool = ffmpegTools.find(t => t.name === toolName);
    if (tool && tool.system_path) {
      // Directly update the config with the system path
      setConfig(prev => ({
        ...prev,
        [`${toolName}_path`]: tool.system_path
      }));
      setSuccessMessage(``);
    } else {
      setErrors([`System path not found for ${toolName}`]);
    }
  };

  const downloadWhisperCliTool = async (toolName: string) => {
    setDownloadingWhisperTool(toolName);
    try {
      const response = await fetch(addTimestamp('/api/whisper-cli/download'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ tool_name: toolName }),
      });

      if (response.ok) {
        // Refresh tools list after download
        await fetchWhisperCliTools();
        await fetchCurrentConfig();
        setSuccessMessage(`${toolName} downloaded successfully`);
      } else {
        const error = await response.json();
        setErrors([error.message || `Failed to download ${toolName}`]);
      }
    } catch (error: any) {
      console.error('Error downloading tool:', error);
      setErrors([error.message || `Failed to download ${toolName}`]);
    } finally {
      setDownloadingWhisperTool(null);
    }
  };

  const useDownloadedWhisperCliTool = async (toolName: string) => {
    try {
      const response = await fetch(addTimestamp('/api/whisper-cli/use-downloaded'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ tool_name: toolName }),
      });

      if (response.ok) {
        await fetchCurrentConfig();
        await fetchWhisperCliTools();
        setSuccessMessage(`Now using downloaded ${toolName}`);
      } else {
        const error = await response.json();
        setErrors([error.message || `Failed to use downloaded ${toolName}`]);
      }
    } catch (error: any) {
      console.error('Error setting tool path:', error);
      setErrors([error.message || `Failed to use downloaded ${toolName}`]);
    }
  };

  const useAutoDetectionForWhisperCli = async (toolName: string) => {
    const tool = whisperCliTools.find(t => t.name === toolName);
    if (tool && tool.system_path) {
      // Directly update the config with the system path
      setConfig(prev => ({
        ...prev,
        [`${toolName}_path`]: tool.system_path
      }));
      setSuccessMessage(``);
    } else {
      setErrors([`System path not found for ${toolName}`]);
    }
  };

  const fetchCurrentConfig = async () => {
    setIsLoading(true);
    try {
      const response = await fetch(addTimestamp('/config'), {
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
            ffprobe_path: data.config.ffprobe_path || '',
            nonlocal_password: data.config.nonlocal_password || ''
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

  const handleBack = () => {
    if (onClose) {
      onClose();
    } else {
      navigate('/');
    }
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

  const handleSave = async (shouldClose: boolean = false) => {
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

    if (config.nonlocal_password !== undefined) {
      submitData.nonlocal_password = config.nonlocal_password;
    }

    try {
      const response = await fetch(addTimestamp('/config'), {
        method: 'POST',
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(submitData)
      });

      const result = await response.json();

      if (response.ok && result.success) {
        setSuccessMessage('Configuration saved successfully!');
        if (shouldClose) {
          setTimeout(() => {
            if (onClose) {
              onClose();
            } else {
              navigate('/');
            }
          }, 1000);
        }
      } else {
        setErrors(result.errors || [result.message || 'Failed to save configuration']);
      }
    } catch {
      setErrors(['Network error: Failed to save configuration']);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="h-full overflow-auto">
      {isLoading ? (
        <div className="text-center py-8">
          <div className="text-lg text-muted-foreground">Loading configuration...</div>
        </div>
      ) : (
        <div className="p-6 h-full">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-lg font-semibold">Configuration</h2>
            {onClose && (
              <button
                onClick={handleBack}
                className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                <ChevronLeft className="h-4 w-4" />
                Close
              </button>
            )}
          </div>
                
                <form onSubmit={(e) => { e.preventDefault(); handleSave(false); }} className="space-y-6">
                  {/* Watch Directories */}
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

                  {/* Whisper-CLI Tools Section */}
                  <div className="space-y-4">
                    <h3 className="text-sm font-semibold text-foreground">Whisper-CLI Tool</h3>
                    
                    {whisperCliTools.map(tool => (
                      <div key={tool.name} className="border border-border rounded-lg p-4 space-y-3">
                        <div className="flex items-center justify-between">
                          <h4 className="text-sm font-medium text-foreground">Whisper CLI</h4>
                          <span className="text-xs text-muted-foreground">Platform: {tool.platform}</span>
                        </div>

                        <div className="space-y-2">
                          {/* Status Information */}
                          <div className="text-xs space-y-1">
                            {tool.system_available && (
                              <div className="flex items-center gap-2 text-green-600">
                                <CheckCircle className="h-3 w-3" />
                                Available in system PATH: {tool.system_path}
                              </div>
                            )}
                            {tool.downloaded && (
                              <div className="flex items-center gap-2 text-green-600">
                                <CheckCircle className="h-3 w-3" />
                                Downloaded to: {tool.downloaded_path}
                              </div>
                            )}
                            {!tool.system_available && !tool.downloaded && (
                              <div className="flex items-center gap-2 text-yellow-600">
                                <AlertCircle className="h-3 w-3" />
                                Not found in system PATH or downloads
                              </div>
                            )}
                          </div>

                          {/* Current Path Input */}
                          <div>
                            <label htmlFor="whispercli_path" className="block text-xs font-medium text-foreground mb-1">
                              Current Path
                            </label>
                            <input
                              type="text"
                              id="whispercli_path"
                              value={config.whispercli_path}
                              onChange={(e) => handleInputChange('whispercli_path', e.target.value)}
                              placeholder="/path/to/whisper-cli"
                              className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent text-sm"
                              required
                            />
                          </div>

                          {/* Action Buttons */}
                          <div className="flex flex-wrap gap-2">
                            {!tool.downloaded && (
                              <button
                                type="button"
                                onClick={() => downloadWhisperCliTool(tool.name)}
                                disabled={downloadingWhisperTool !== null}
                                className="flex items-center gap-2 px-3 py-1.5 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                              >
                                <Download className="h-3 w-3" />
                                {downloadingWhisperTool === tool.name ? 'Downloading...' : 'Download'}
                              </button>
                            )}
                            
                            {tool.downloaded && (
                              <button
                                type="button"
                                onClick={() => useDownloadedWhisperCliTool(tool.name)}
                                className="px-3 py-1.5 text-xs bg-green-600 text-white rounded hover:bg-green-700"
                              >
                                Use Downloaded Version
                              </button>
                            )}
                            
                            {tool.system_available && (
                              <button
                                type="button"
                                onClick={() => useAutoDetectionForWhisperCli(tool.name)}
                                className="px-3 py-1.5 text-xs bg-gray-600 text-white rounded hover:bg-gray-700"
                              >
                                Use System Version
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>

                  {/* Model Selection */}
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

                  {/* FFmpeg Tools Section */}
                  <div className="space-y-4">
                    <h3 className="text-sm font-semibold text-foreground">FFmpeg Tools</h3>
                    
                    {ffmpegTools.map(tool => (
                      <div key={tool.name} className="border border-border rounded-lg p-4 space-y-3">
                        <div className="flex items-center justify-between">
                          <h4 className="text-sm font-medium text-foreground capitalize">{tool.name}</h4>
                          <span className="text-xs text-muted-foreground">Platform: {tool.platform}</span>
                        </div>

                        <div className="space-y-2">
                          {/* Status Information */}
                          <div className="text-xs space-y-1">
                            {tool.system_available && (
                              <div className="flex items-center gap-2 text-green-600">
                                <CheckCircle className="h-3 w-3" />
                                Available in system PATH: {tool.system_path}
                              </div>
                            )}
                            {tool.downloaded && (
                              <div className="flex items-center gap-2 text-green-600">
                                <CheckCircle className="h-3 w-3" />
                                Downloaded to: {tool.downloaded_path}
                              </div>
                            )}
                            {!tool.system_available && !tool.downloaded && (
                              <div className="flex items-center gap-2 text-yellow-600">
                                <AlertCircle className="h-3 w-3" />
                                Not found in system PATH or downloads
                              </div>
                            )}
                          </div>

                          {/* Current Path Input */}
                          <div>
                            <label htmlFor={`${tool.name}_path`} className="block text-xs font-medium text-foreground mb-1">
                              Current Path
                            </label>
                            <input
                              type="text"
                              id={`${tool.name}_path`}
                              value={config[`${tool.name}_path` as keyof ConfigData] as string}
                              onChange={(e) => handleInputChange(`${tool.name}_path` as keyof Omit<ConfigData, 'watch_directories'>, e.target.value)}
                              placeholder={`/path/to/${tool.name}`}
                              className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent text-sm"
                              required
                            />
                          </div>

                          {/* Action Buttons */}
                          <div className="flex flex-wrap gap-2">
                            {!tool.downloaded && (
                              <button
                                type="button"
                                onClick={() => downloadFFmpegTool(tool.name)}
                                disabled={downloadingTool !== null}
                                className="flex items-center gap-2 px-3 py-1.5 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                              >
                                <Download className="h-3 w-3" />
                                {downloadingTool === tool.name ? 'Downloading...' : 'Download'}
                              </button>
                            )}
                            
                            {tool.downloaded && (
                              <button
                                type="button"
                                onClick={() => useDownloadedFFmpegTool(tool.name)}
                                className="px-3 py-1.5 text-xs bg-green-600 text-white rounded hover:bg-green-700"
                              >
                                Use Downloaded Version
                              </button>
                            )}
                            
                            {tool.system_available && (
                              <button
                                type="button"
                                onClick={() => useAutoDetectionForTool(tool.name)}
                                className="px-3 py-1.5 text-xs bg-gray-600 text-white rounded hover:bg-gray-700"
                              >
                                Use System Version
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>

                  {/* API Password */}
                  <div>
                    <label htmlFor="nonlocal_password" className="block text-sm font-medium text-foreground mb-1">
                      API Password (optional)
                    </label>
                    <input
                      type="password"
                      id="nonlocal_password"
                      value={config.nonlocal_password || ''}
                      onChange={(e) => handleInputChange('nonlocal_password', e.target.value)}
                      placeholder="Set or clear API password (leave blank to disable)"
                      className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      If set, all non-local API requests require this password (via Basic Auth or cookie).
                    </p>
                  </div>

                  {/* Error Messages */}
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

                  {/* Success Message */}
                  {successMessage && (
                    <div className="bg-accent/10 border border-accent/20 rounded-md p-3">
                      <div className="text-sm text-accent-foreground">
                        {successMessage}
                      </div>
                    </div>
                  )}

                  {/* Action Buttons */}
                  <div className="flex gap-3 pt-2">
                    <button
                      type="submit"
                      disabled={isSubmitting || !hasAllConfigValues()}
                      className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {isSubmitting ? 'Saving...' : 'Save'}
                    </button>
                    
                    <button
                      type="button"
                      onClick={() => handleSave(true)}
                      disabled={isSubmitting || !hasAllConfigValues()}
                      className="px-4 py-2 bg-secondary text-secondary-foreground rounded-md hover:bg-secondary/80 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {isSubmitting ? 'Saving...' : 'Save and Close'}
                    </button>
                  </div>
                </form>
        </div>
      )}
    </div>
  );
}