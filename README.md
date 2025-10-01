# AI Agent with Rig + Tauri

A conversational AI agent desktop/mobile application built with Tauri and the Rig framework, featuring real-time streaming responses and Model Context Protocol (MCP) integration for web search.

## Overview

Chat interface powered by OpenAI's GPT-4 with streaming responses and extensible tool support via MCP servers.

**Tech Stack:**
- **Frontend**: Preact, TypeScript, Tailwind CSS
- **Backend**: Rust, Tauri, Rig, RMCP
- **Platform**: iOS (Xcode simulator), Desktop (macOS, Windows, Linux)
- **MCP Server**: Brave Search (Docker)

## Features

- Real-time streaming chat responses
- Custom tools (current time, web search)
- MCP integration for extensible capabilities
- Cross-platform support (iOS + Desktop)

## Quick Start

### Prerequisites
- Rust (latest stable)
- Node.js (v18+) and pnpm
- Xcode (for iOS development)
- Docker (for Brave MCP server)
- OpenAI API key
- Brave API key (optional)

### Setup

1. **Configure environment**

   Create `ai-agent-tauri/src-tauri/.env`:
   ```env
   OPENAI_API_KEY=sk-proj-...
   BRAVE_API_KEY=BSA...
   MCP_SERVER_URL=http://localhost:8081
   ```

2. **Start Brave MCP server**
   ```bash
   docker run -p 8081:8081 brave-mcp-server
   ```

3. **Run the app**

   For desktop:
   ```bash
   cd ai-agent-tauri
   pnpm install
   pnpm tauri dev
   ```

   For iOS:
   ```bash
   cd ai-agent-tauri
   pnpm install
   pnpm tauri ios dev
   ```

## Project Structure

```
ai-agent-rig/
├── ai-agent-rig/          # CLI example (simple Rig demo)
└── ai-agent-tauri/        # Main application
    ├── src/               # Preact frontend
    └── src-tauri/         # Rust backend
```

## MCP Integration

The app connects to MCP servers for extended capabilities. The Brave MCP server provides web search functionality. If the MCP server is unavailable, the app runs without web search.

## License

MIT
