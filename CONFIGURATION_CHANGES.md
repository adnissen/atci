# Configuration System Changes

This document outlines the changes made to move Autotranscript from hard-coded configuration to a flexible file-based and web UI configurable system.

## Overview

The application has been updated to use a `.atconfig` file for configuration instead of hard-coded values in `config.exs`. Users can now configure the application through either:

1. **File-based configuration**: Create a `.atconfig` file in JSON format
2. **Web UI configuration**: Use the setup form in the web interface

## Changes Made

### 1. Configuration Manager (`lib/autotranscript/config_manager.ex`)

**New module** that handles reading and writing configuration:

- **`get_config/0`**: Reads configuration from `.atconfig` file
- **`save_config/1`**: Saves configuration to `.atconfig` file  
- **`config_complete?/1`**: Validates if all required configuration is present
- **`get_config_value/1`**: Gets a specific configuration value

**Configuration file lookup priority**:
1. `.atconfig` in current directory (where program is run)
2. `.atconfig` in user's home directory (`~/.atconfig`)

### 2. Configuration Controller (`lib/autotranscript/web/controllers/config_controller.ex`)

**New controller** with API endpoints:

- **GET `/config`**: Returns current configuration status and values
- **POST `/config`**: Updates configuration with validation

**API Response format**:
```json
{
  "config": {
    "watch_directory": "/path/to/videos",
    "whispercli_path": "/path/to/whisper-cli", 
    "model_path": "/path/to/model.bin"
  },
  "is_complete": true
}
```

### 3. Updated Application Modules

**Modified files to use new configuration system**:

- `lib/autotranscript/transcriber/transcriber.ex`
- `lib/autotranscript/transcriber/video_processor.ex`
- `lib/autotranscript/web/controllers/transcript_controller.ex`

**Key changes**:
- Replaced `Application.get_env(:autotranscript, :key)` with `Autotranscript.ConfigManager.get_config_value("key")`
- Added null/empty checks for configuration values
- Added proper error handling when configuration is missing

### 4. Router Updates (`lib/autotranscript/web/router.ex`)

**Added configuration routes**:
```elixir
get "/config", ConfigController, :show
post "/config", ConfigController, :update
```

### 5. Frontend Configuration Setup (`frontend/src/components/ConfigSetup.tsx`)

**New React component** that provides a configuration form:

- **File/directory path inputs** for all three required values
- **Real-time validation** with error display
- **Automatic redirect** to main app after successful configuration

**Features**:
- Form validation with helpful error messages
- Loading states during configuration save
- Responsive design matching the app's theme

### 6. App Integration (`frontend/src/App.tsx`)

**Updated main app** to check configuration before showing interface:

- **Configuration status checking** on app load
- **Conditional rendering**: Shows setup form if configuration incomplete
- **Loading state** while checking configuration
- **Updated useEffect hooks** to only run when configuration is complete

## Configuration File Format

The `.atconfig` file uses JSON format:

```json
{
  "watch_directory": "/path/to/your/videos",
  "whispercli_path": "/path/to/whisper-cli",
  "model_path": "/path/to/your/whisper.cpp/model.bin"
}
```

## Usage Instructions

### First Time Setup

1. **Start the application**
2. **Open web interface** - you'll see the configuration setup form
3. **Fill in the required paths**:
   - **Watch Directory**: Directory containing MP4 files to transcribe
   - **Whisper CLI Path**: Path to the whisper.cpp CLI executable
   - **Model Path**: Path to the Whisper model file (.bin)
4. **Click "Save Configuration"**
5. **Application will automatically redirect** to main interface

### Manual Configuration

Create a `.atconfig` file in either:
- The directory where you run the application, or
- Your home directory (`~/.atconfig`)

Example:
```bash
# In application directory
cat > .atconfig << EOF
{
  "watch_directory": "/home/user/videos",
  "whispercli_path": "/usr/local/bin/whisper", 
  "model_path": "/home/user/models/ggml-base.bin"
}
EOF
```

### Configuration Validation

The system validates that:
- All three paths are provided and non-empty
- **watch_directory** exists and is a directory
- **whispercli_path** exists and is a file
- **model_path** exists and is a file

## Error Handling

### Application Behavior When Configuration Missing

- **Transcription services** log warnings and skip processing
- **Web API endpoints** return 503 Service Unavailable
- **Frontend** shows configuration setup form

### Graceful Degradation

- Application starts successfully even without configuration
- Services that require configuration paths fail gracefully
- User gets clear guidance on what needs to be configured

## Configuration Priority

1. **.atconfig in current directory** (highest priority)
2. **.atconfig in home directory** (fallback)
3. **No configuration** (shows setup form)

## Security Considerations

- Configuration file contains file paths only (no secrets)
- Validation ensures paths exist before saving
- No remote configuration sources (local files only)

## Migration from Old System

### Old System (config.exs)
```elixir
config :autotranscript,
  watch_directory: "/path/to/your/videos",
  whispercli_path: "/path/to/whisper-cli",
  model_path: "/path/to/your/whisper.cpp/model.bin"
```

### New System (.atconfig)
```json
{
  "watch_directory": "/path/to/your/videos",
  "whispercli_path": "/path/to/whisper-cli", 
  "model_path": "/path/to/your/whisper.cpp/model.bin"
}
```

## Technical Details

### File Reading Strategy

1. **Current directory check**: `./atconfig`
2. **Home directory check**: `~/.atconfig`  
3. **File parsing**: JSON.decode with error handling
4. **Caching**: Configuration read on each access (no caching for dynamic updates)

### Error Recovery

- **File not found**: Returns empty config, triggers setup form
- **Invalid JSON**: Logs error, returns empty config
- **Permission errors**: Logs error, returns empty config
- **Validation failures**: Returns specific error messages

## API Endpoints

### GET /config
Returns current configuration status and values.

**Response**:
```json
{
  "config": { ... },
  "is_complete": boolean
}
```

### POST /config  
Updates configuration with validation.

**Request**:
```json
{
  "watch_directory": "string",
  "whispercli_path": "string",
  "model_path": "string"
}
```

**Response** (success):
```json
{
  "success": true,
  "message": "Configuration saved successfully",
  "config": { ... }
}
```

**Response** (error):
```json
{
  "success": false,
  "message": "Invalid configuration",
  "errors": ["error1", "error2"]
}
```

## Testing

### Manual Testing Steps

1. **Test fresh installation**: Start app without .atconfig
2. **Test configuration form**: Fill out and submit setup form
3. **Test validation**: Try invalid paths, empty values
4. **Test file precedence**: Create .atconfig in both current and home directories
5. **Test configuration reload**: Modify .atconfig file and restart app

### Configuration Scenarios

- **No configuration file**: Should show setup form
- **Incomplete configuration**: Should show setup form with errors
- **Valid configuration**: Should show main application interface
- **Invalid file paths**: Should show validation errors
- **Permission issues**: Should handle gracefully with error messages

## Future Enhancements

### Potential Improvements

1. **Configuration hot-reload**: Watch .atconfig file for changes
2. **Configuration backup**: Save backup of working configuration
3. **Multiple configuration profiles**: Support for different environments
4. **Configuration validation UI**: Real-time path validation in web form
5. **Configuration export/import**: Easy sharing of configuration between systems

## Troubleshooting

### Common Issues

1. **"Watch directory not configured"**: Check .atconfig file exists and is valid JSON
2. **"Whisper CLI not found"**: Verify whispercli_path points to correct executable
3. **"Model file not found"**: Ensure model_path points to valid .bin file
4. **Setup form not working**: Check browser console for API errors
5. **Configuration not saving**: Check file permissions in current directory

### Debug Steps

1. **Check configuration file**: `cat .atconfig`
2. **Verify JSON format**: Use online JSON validator
3. **Test file permissions**: Try creating/editing .atconfig manually
4. **Check application logs**: Look for configuration-related errors
5. **Test API endpoints**: Use curl to test /config endpoints

## Conclusion

The new configuration system provides:
- **Flexibility**: File-based or web UI configuration
- **User-friendly setup**: No need to edit code files
- **Robust validation**: Prevents common configuration errors
- **Graceful degradation**: App works even with missing configuration
- **Clear error messages**: Users know exactly what to fix

This system makes Autotranscript much easier to deploy and configure for end users while maintaining backward compatibility and robust error handling.