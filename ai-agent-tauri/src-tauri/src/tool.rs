use chrono::Local;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Error, Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCurrentTimeArgs {}

#[derive(Clone)]
pub struct GetCurrentTime;
impl Tool for GetCurrentTime {
    const NAME: &'static str = "get_current_time";
    type Error = Error;
    type Args = GetCurrentTimeArgs;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the current local time".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let current_time = Local::now();
        let formatted_time = current_time.format("%Y-%m-%d %H:%M:%S").to_string();
        Ok(json!({"current_time":formatted_time}))
    }
}
