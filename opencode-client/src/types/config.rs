//! `Config` — the opencode configuration-file model, mirroring the `Config`
//! schema of the native server (`GET /doc`, OpenAPI). The UI does not consume
//! it yet, but it is modelled faithfully (including every schema reachable from
//! `Config`) so the client's type surface stays in agreement with the server.
//!
//! Convention: enums and structs are exhaustive — no `Unknown` catch-all — so
//! that a growing server schema forces a deliberate decision (see AGENTS.md,
//! "Принципы"). Free-form leaves that the schema itself declares open use
//! `serde_json::Value`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A JSON object with arbitrary values, as used by open-ended schema leaves.
pub type Value = serde_json::Value;
/// A JSON object with string values.
pub type StringMap = BTreeMap<String, String>;
/// A JSON object with boolean values.
pub type BoolMap = BTreeMap<String, bool>;

/// Log level (`logLevel`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Effect of a permission policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// A single permission action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionActionConfig {
    Ask,
    Allow,
    Deny,
}

/// @deprecated Always uses stretch layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LayoutConfig {
    Auto,
    Stretch,
}

/// Server configuration for `opencode serve` and web commands.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mdns: Option<bool>,
    #[serde(default, rename = "mdnsDomain", skip_serializing_if = "Option::is_none")]
    pub mdns_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cors: Option<Vec<String>>,
}

/// Image-attachment handling.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImageAttachmentConfig {
    #[serde(default, rename = "auto_resize", skip_serializing_if = "Option::is_none")]
    pub auto_resize: Option<bool>,
    #[serde(default, rename = "max_width", skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u64>,
    #[serde(default, rename = "max_height", skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u64>,
    #[serde(
        default,
        rename = "max_base64_bytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_base64_bytes: Option<u64>,
}

/// Attachment settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AttachmentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageAttachmentConfig>,
}

/// OAuth configuration for a remote MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct McpOAuthConfig {
    #[serde(default, rename = "clientId", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, rename = "clientSecret", skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, rename = "callbackPort", skip_serializing_if = "Option::is_none")]
    pub callback_port: Option<u32>,
    #[serde(default, rename = "redirectUri", skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// Discriminant of a local MCP server config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpLocalType {
    Local,
}

/// Discriminant of a remote MCP server config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpRemoteType {
    Remote,
}

/// A locally spawned MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpLocalConfig {
    #[serde(rename = "type")]
    pub type_: McpLocalType,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: StringMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// A remote MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRemoteConfig {
    #[serde(rename = "type")]
    pub type_: McpRemoteType,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: StringMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<McpOAuth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// `oauth` is either an object or the literal `false` (to disable OAuth).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum McpOAuth {
    Config(McpOAuthConfig),
    Disabled(bool),
}

/// A disabled/overridden MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpEnabledConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Any MCP override entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum McpConfig {
    Local(McpLocalConfig),
    Remote(McpRemoteConfig),
    Enabled(McpEnabledConfig),
}

/// A git reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigV2ReferenceGit {
    #[serde(default)]
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// A local-path reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigV2ReferenceLocal {
    #[serde(default)]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// A reference is a bare string or one of the structured reference forms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ReferenceValue {
    Simple(String),
    Git(ConfigV2ReferenceGit),
    Local(ConfigV2ReferenceLocal),
}

/// A single experimental policy entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigV2ExperimentalPolicy {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub effect: Option<PolicyEffect>,
    #[serde(default)]
    pub resource: String,
}

/// A numeric timeout that can be disabled with `false`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TimeoutValue {
    Millis(u64),
    Disabled(bool),
}

/// Provider request options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderOptions {
    #[serde(default, rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, rename = "enterpriseUrl", skip_serializing_if = "Option::is_none")]
    pub enterprise_url: Option<String>,
    #[serde(default, rename = "setCacheKey", skip_serializing_if = "Option::is_none")]
    pub set_cache_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<TimeoutValue>,
    #[serde(default, rename = "headerTimeout", skip_serializing_if = "Option::is_none")]
    pub header_timeout: Option<TimeoutValue>,
    #[serde(default, rename = "chunkTimeout", skip_serializing_if = "Option::is_none")]
    pub chunk_timeout: Option<TimeoutValue>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Cost for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelCost {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_over_200k: Option<ProviderModelCostTier>,
}

/// Cost tier above the 200k context window.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelCostTier {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

/// Token limits of a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelLimit {
    #[serde(default)]
    pub context: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
}

/// Input/output modalities of a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelModalities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<Modality>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<Modality>>,
}

/// A single modality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Audio,
    Image,
    Video,
    Pdf,
}

/// Lifecycle status of a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderModelStatus {
    Alpha,
    Beta,
    Deprecated,
    Active,
}

/// Nested provider descriptor of a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelProvider {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

/// Variant-specific overrides of a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelVariant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A model entry in a provider's catalogue.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderModelConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    #[serde(default, rename = "tool_call", skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ProviderModelCost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<ProviderModelLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ProviderModelModalities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProviderModelStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderModelProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: StringMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variants: BTreeMap<String, ProviderModelVariant>,
}

/// A provider definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blacklist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ProviderOptions>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<String, ProviderModelConfig>,
}

/// An object of named permission rules.
pub type PermissionObjectConfig = BTreeMap<String, PermissionActionConfig>;

/// A single permission rule: an action or a map of actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissionRuleConfig {
    Action(PermissionActionConfig),
    Object(PermissionObjectConfig),
}

/// Named permission rules (keys may extend the known set).
pub type PermissionRuleConfigMap = BTreeMap<String, PermissionRuleConfig>;

/// The `permission` block: a single action or a set of named rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissionConfig {
    Action(PermissionActionConfig),
    Rules(PermissionRuleConfigMap),
}

/// A user-defined command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandConfig {
    #[serde(default)]
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
}

/// Skill discovery settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SkillsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
}

/// File-watcher settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WatcherConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
}

/// Enterprise settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnterpriseConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Tool-output truncation limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ToolOutputConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
}

/// Auto-compaction settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompactionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prune: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail_turns: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserve_recent_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved: Option<u64>,
}

/// Experimental feature flags and policies.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExperimentalConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_paste_summary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_tool: Option<bool>,
    #[serde(default, rename = "openTelemetry", skip_serializing_if = "Option::is_none")]
    pub open_telemetry: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_loop_on_deny: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<ConfigV2ExperimentalPolicy>>,
}

/// A plugin entry: a name, or a `[name, options]` tuple.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PluginValue {
    Name(String),
    Tuple(String, Value),
}

/// A formatter override.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FormatterItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: StringMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

/// `formatter`: `true`, `false`, or a map of overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FormatterConfig {
    Enabled(bool),
    Config(BTreeMap<String, FormatterItem>),
}

/// An LSP server override.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LspServerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: StringMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialization: Option<Value>,
}

/// `lsp`: `true`, `false`, or a map of overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum LspConfig {
    Enabled(bool),
    Config(BTreeMap<String, LspServerConfig>),
}

/// `autoupdate`: `true`/`false`, or the string `notify`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AutoupdateConfig {
    Enabled(bool),
    Mode(AutoupdateMode),
}

/// The only string mode for `autoupdate`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AutoupdateMode {
    Notify,
}

/// `share`: when to publish sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ShareConfig {
    Manual,
    Auto,
    Disabled,
}

/// An agent definition (`agent` / `mode`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BoolMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<AgentMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,
    #[serde(default, rename = "maxSteps", skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfig>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Agent mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    Subagent,
    Primary,
    All,
}

/// Root configuration object (a `opencode.json` / `opencode.jsonc` file).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, rename = "logLevel", skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub command: BTreeMap<String, CommandConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<SkillsConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub references: BTreeMap<String, ReferenceValue>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reference: BTreeMap<String, ReferenceValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher: Option<WatcherConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Vec<PluginValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<ShareConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoshare: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<AutoupdateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_providers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_providers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mode: BTreeMap<String, AgentConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent: BTreeMap<String, AgentConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub provider: BTreeMap<String, ProviderConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mcp: BTreeMap<String, McpConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<LspConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BoolMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<AttachmentConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise: Option<EnterpriseConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<ToolOutputConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalConfig>,
}
