use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Who submitted this proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalSender {
	Agent,
	Mcp,
	Webhook,
	Watcher,
	Relay,
	UserSelf,
}

impl ProposalSender {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Agent => "agent",
			Self::Mcp => "mcp",
			Self::Webhook => "webhook",
			Self::Watcher => "watcher",
			Self::Relay => "relay",
			Self::UserSelf => "self",
		}
	}
}

impl std::str::FromStr for ProposalSender {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"agent" => Ok(Self::Agent),
			"mcp" => Ok(Self::Mcp),
			"webhook" => Ok(Self::Webhook),
			"watcher" => Ok(Self::Watcher),
			"relay" => Ok(Self::Relay),
			"self" => Ok(Self::UserSelf),
			_ => Err(format!("unknown proposal sender: {s}")),
		}
	}
}

impl std::fmt::Display for ProposalSender {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// What action the proposal wants to take
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalAction {
	CreateNode,
	UpdateNode,
	DeleteNode,
	SuggestTag,
	SuggestLink,
	ScheduleReminder,
	Custom(String),
}

impl ProposalAction {
	pub fn as_str(&self) -> &str {
		match self {
			Self::CreateNode => "create_node",
			Self::UpdateNode => "update_node",
			Self::DeleteNode => "delete_node",
			Self::SuggestTag => "suggest_tag",
			Self::SuggestLink => "suggest_link",
			Self::ScheduleReminder => "schedule_reminder",
			Self::Custom(s) => s,
		}
	}
}

impl std::str::FromStr for ProposalAction {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"create_node" => Ok(Self::CreateNode),
			"update_node" => Ok(Self::UpdateNode),
			"delete_node" => Ok(Self::DeleteNode),
			"suggest_tag" => Ok(Self::SuggestTag),
			"suggest_link" => Ok(Self::SuggestLink),
			"schedule_reminder" => Ok(Self::ScheduleReminder),
			other => Ok(Self::Custom(other.to_string())),
		}
	}
}

impl std::fmt::Display for ProposalAction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Current state of the proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
	Pending,
	Approved,
	Rejected,
	Expired,
	AutoApproved,
}

impl ProposalState {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Pending => "pending",
			Self::Approved => "approved",
			Self::Rejected => "rejected",
			Self::Expired => "expired",
			Self::AutoApproved => "auto_approved",
		}
	}
}

impl std::str::FromStr for ProposalState {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"pending" => Ok(Self::Pending),
			"approved" => Ok(Self::Approved),
			"rejected" => Ok(Self::Rejected),
			"expired" => Ok(Self::Expired),
			"auto_approved" => Ok(Self::AutoApproved),
			_ => Err(format!("unknown proposal state: {s}")),
		}
	}
}

impl std::fmt::Display for ProposalState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// A proposal in the Exchange Inbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
	pub id: Uuid,
	pub node_id: Option<Uuid>,
	pub target_node_id: Option<Uuid>,
	pub sender: ProposalSender,
	pub action: ProposalAction,
	pub state: ProposalState,
	pub confidence: f32,
	pub diff_preview: Option<String>,
	pub payload: HashMap<String, serde_json::Value>,
	pub created_at: DateTime<Utc>,
	pub updated_at: Option<DateTime<Utc>>,
	pub resolved_at: Option<DateTime<Utc>>,
}

impl Proposal {
	pub fn new(sender: ProposalSender, action: ProposalAction) -> Self {
		Self {
			id: Uuid::now_v7(),
			node_id: None,
			target_node_id: None,
			sender,
			action,
			state: ProposalState::Pending,
			confidence: 0.5,
			diff_preview: None,
			payload: HashMap::new(),
			created_at: Utc::now(),
			updated_at: None,
			resolved_at: None,
		}
	}

	pub fn with_target(mut self, target_node_id: Uuid) -> Self {
		self.target_node_id = Some(target_node_id);
		self
	}

	pub fn with_confidence(mut self, confidence: f32) -> Self {
		self.confidence = confidence.clamp(0.0, 1.0);
		self
	}

	pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
		self.diff_preview = Some(diff.into());
		self
	}

	pub fn with_payload(mut self, payload: HashMap<String, serde_json::Value>) -> Self {
		self.payload = payload;
		self
	}
}
