# Environment Setup for Background Agents

## Current Environment State

### System Requirements
- Elixir/Erlang for Phoenix backend
- Node.js 20+ for React frontend
- Mix for Elixir dependency management
- npm/pnpm for frontend dependencies

### Manual Setup Steps Completed
1. `mix deps.get` - Installed Elixir dependencies
2. `cd frontend && npm install` - Installed frontend dependencies
3. Configured Phoenix server on port 4000
4. Configured Vite dev server

### Key Directories
- `/` - Phoenix/Elixir backend
- `/frontend` - React/Vite frontend
- `/lib/autotranscript` - Main application code
- `/config` - Phoenix configuration

### Ports Used
- 4000: Phoenix server
- 5173: Vite dev server (if running dev mode)

### Environment Variables
(Add any environment variables you've configured)

## Commands for Fresh Setup
```bash
# Backend setup
mix deps.get

# Frontend setup  
cd frontend
npm install

# Start services
mix phx.server &
cd frontend && npm run build -- --watch
```

## Notes
- This environment is optimized for background agent development
- Dependencies are automatically installed via .cursor/environment.json
- Terminal commands are available via Cursor command palette