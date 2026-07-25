use serde_json::Value;

/// MCP-style tool contract. Each capability is one module implementing this.
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    #[allow(dead_code)] // registered via tool_definitions() for the agent
    fn description(&self) -> &'static str;
    #[allow(dead_code)] // registered via tool_definitions() for the agent
    fn input_schema(&self) -> Value;
    fn execute(&self, input: Value) -> Result<String, String>;
}
