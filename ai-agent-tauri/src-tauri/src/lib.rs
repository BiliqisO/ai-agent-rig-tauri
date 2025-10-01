// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rig::{
    agent::MultiTurnStreamItem,
    client::{completion::CompletionClientDyn, ProviderClient},
    providers::{self, openai},
    streaming::{StreamingPrompt, StreamedAssistantContent},
};

use std::{sync::Arc, time::Duration};
use tauri::Emitter;
use tokio::sync::Mutex;
use serde::Serialize;
use log::{error, info};
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

    let mut agent = openai_client
            .agent(providers::openai::GPT_4O)
            .preamble("You are a helpful assistant. Use your tools when necessary.")
            .max_tokens(1024)
            .tool(tool::GetCurrentTime);

    match get_connection().await {
        Ok((tools, client)) => {
            info!("✓ Connected to MCP server with {} tools", tools.len());
            for tool in &tools {
                info!("  - Tool: {}", tool.name);
            }
            for tool in tools {
                agent = agent.rmcp_tool(tool, client.clone());
            }
        }
        Err(e) => {
            error!("Failed to connect to MCP server: {}", e);
            error!("Agent will run without web search capability");
        }
    }

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
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

    let server_url = std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let endpoint = format!("{}/mcp", server_url);

    let uri: std::sync::Arc<str> = endpoint.into();
    let config = StreamableHttpClientTransportConfig {
        uri,
        ..Default::default()
    };

    let transport = rmcp::transport::StreamableHttpClientTransport::with_client(
        reqwest::Client::new(),
        config
    );

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "agent-conversation".to_string(),
            version: "0.1.0".to_string(),
            title: None,
            website_url: None,
            icons: None,
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
    dotenv::dotenv().ok();

    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("OPENAI_API_KEY environment variable not set");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![chat_with_agent])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}