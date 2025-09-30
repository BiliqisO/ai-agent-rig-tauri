// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rig::{
    agent::MultiTurnStreamItem,
    client::{completion::CompletionClientDyn, ProviderClient},
    providers::{self, openai},
    streaming::{StreamingPrompt, StreamedAssistantContent},
};
use tauri::Emitter;
use serde::Serialize;
use log::error;
use futures_util::StreamExt;

mod tool;

#[derive(Clone, Serialize)]
struct AgentChunk {
    delta: Option<String>,
    tool_calls: Option<serde_json::Value>,
}
static CONNECTION_POOL: tokio::sync::OnceCell<Arc<Mutex<Option<ConnectionHolder>>>> = tokio::sync::OnceCell::const_new();

struct ConnectionHolder {
    client: rmcp::Peer<rmcp::RoleClient>,
    tools: Vec<rmcp::model::Tool>,
    _service: Box<dyn std::any::Any + Send + Sync>,
}
#[tauri::command]
async fn chat_with_agent(message: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    eprintln!("=== CHAT_WITH_AGENT CALLED ===");
    eprintln!("Received chat message: {}", message);

    if std::env::var("OPENAI_API_KEY").is_err() {
        let error_msg = "OPENAI_API_KEY environment variable not set";
        eprintln!("ERROR: {}", error_msg);
        eprintln!("Current working directory: {:?}", std::env::current_dir());

        let error_chunk = AgentChunk {
            delta: Some(error_msg.to_string()),
            tool_calls: None,
        };

        app_handle.emit("agent-chunk", error_chunk).ok();
        return Err(error_msg.to_string());
    }
    let openai_client = openai::Client::from_env();

    let agent = openai_client
            .agent(providers::openai::GPT_4O)
            .preamble("You are a helpful assistant. Use your tools when necessary.")
            .tool(tool::GetCurrentTime)
    if let Ok((tools, client)) = get_connection().await{
        info!("Using connection with {} tools", tools.len());
        for tool in tools{
            agent = agent.rmcp_tool(tool, client.clone());
        }
    };
    let agent = agent.build();

  

    let mut stream = agent.stream_prompt(&message).await;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(MultiTurnStreamItem::StreamItem(content)) => {
    
                if let StreamedAssistantContent::Text(text) = content {
                    let text_str = text.to_string();
                    eprintln!("Streaming: {}", text_str);

                    let agent_chunk = AgentChunk {
                        delta: Some(text_str),
                        tool_calls: None,
                    };

                    if let Err(e) = app_handle.emit("agent-chunk", agent_chunk) {
                        eprintln!("Failed to emit chunk: {:?}", e);
                    }
                }
            }
            Ok(MultiTurnStreamItem::FinalResponse(final_response)) => {
                eprintln!("Stream complete: {:?}", final_response);
                break;
            }
            Ok(_other) => {
                // Handle other stream item variants
                continue;
            }
            Err(e) => {
                error!("Agent stream error: {:?}", e);
                let error_chunk = AgentChunk {
                    delta: Some(format!("Error: {}", e)),
                    tool_calls: None,
                };
                app_handle.emit("agent-chunk", error_chunk).ok();
                return Err(format!("Stream error: {}", e));
            }
        }
    }

    eprintln!("Stream completed successfully");
    Ok(())
}

async fn get_connection() -> Result<(Vec<rmcp::model::Tool>, rmcp::Peer<rmcp::RoleClient>), Box<dyn std::error::Error + Send + Sync>> {
    let pool = CONNECTION_POOL
        .get_or_init(|| async { Arc::new(Mutex::new(None)) })
        .await;
    let mut guard = pool.lock().await;

    if let Some(holder) = guard.as_ref() {
        if tokio::time::timeout(Duration::from_secs(2), holder.client.list_tools(Default::default())).await.is_ok() {
            return Ok((holder.tools.clone(), holder.client.clone()));
        }
        *guard = None;
    }

    let holder = create_connection().await?;
    let tools = holder.tools.clone();
    let client = holder.client.clone();
    *guard = Some(holder);
    Ok((tools, client))
}
async fn create_connection() -> Result<ConnectionHolder, Box<dyn std::error::Error + Send + Sync>> {
    use rmcp::{model::{ClientCapabilities, ClientInfo, Implementation}, ServiceExt};

    let server_url = std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let endpoint = format!("{}/mcp", server_url.replace("localhost", "192.168.1.4"));
    let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(endpoint.as_str());

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "agent-conversation".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let service = client_info.serve(transport).await?;
    let client = service.peer().clone();
    let mut tools = tokio::time::timeout(Duration::from_secs(10), client.list_tools(Default::default())).await??.tools;

    for tool in &mut tools {
        let mut schema = (*tool.input_schema).clone();
        if let Some(props) = schema.get("properties") {
            if let Some(props_obj) = props.as_object() {
                let required: Vec<String> = props_obj.keys().cloned().collect();
                schema.insert("required".to_string(), serde_json::json!(required));
            }
        }
        tool.input_schema = std::sync::Arc::new(schema);
    }

    Ok(ConnectionHolder {
        client,
        tools,
        _service: Box::new(service),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("=== APP STARTING ===");
    eprintln!("Current directory: {:?}", std::env::current_dir());

    // Try to load .env - dotenv should work from src-tauri directory
    match dotenv::dotenv() {
        Ok(path) => eprintln!("✓ Loaded .env from: {:?}", path),
        Err(e) => {
            eprintln!("dotenv::dotenv() failed: {:?}", e);
            eprintln!("Attempting manual .env load...");

            // Manual fallback - try to read and parse .env file
            let paths = [
                ".env",
                "src-tauri/.env",
                "../.env",
                "../../.env",
            ];

            let mut loaded = false;
            for path in &paths {
                eprintln!("  Checking: {}", path);
                if let Ok(contents) = std::fs::read_to_string(path) {
                    eprintln!("  ✓ Found .env at: {}", path);
                    for line in contents.lines() {
                        let line = line.trim();
                        // Skip empty lines and comments
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if line.starts_with("OPENAI_API_KEY") {
                            if let Some((_, value)) = line.split_once('=') {
                                // Remove quotes and trim whitespace
                                let key = value.trim().trim_matches('"').trim_matches('\'');
                                if !key.is_empty() {
                                    std::env::set_var("OPENAI_API_KEY", key);
                                    eprintln!("  ✓ Set OPENAI_API_KEY from {} (length: {})", path, key.len());
                                    loaded = true;
                                    break;
                                }
                            }
                        }
                    }
                    if loaded {
                        break;
                    }
                }
            }

            if !loaded {
                eprintln!("  ✗ Could not find .env file in any expected location");
            }
        }
    }

    // Verify API key is loaded and trim any quotes if dotenv loaded them
    match std::env::var("OPENAI_API_KEY") {
        Ok(key) => {
            // If the key has quotes, remove them
            let cleaned_key = key.trim_matches('"').trim_matches('\'');
            if cleaned_key != key {
                eprintln!("  Cleaning quotes from OPENAI_API_KEY...");
                std::env::set_var("OPENAI_API_KEY", cleaned_key);
            }

            let final_key = std::env::var("OPENAI_API_KEY").unwrap();
            let len = final_key.len();
            if len > 11 {
                eprintln!("✓✓✓ OPENAI_API_KEY READY: {}...{} (length: {})", &final_key[..7], &final_key[len-4..], len);
            } else {
                eprintln!("✓ OPENAI_API_KEY set (length: {})", len);
            }
        }
        Err(_) => {

            eprintln!("Please set OPENAI_API_KEY in your .env file or environment variables.");
    
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![chat_with_agent])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}