use std::sync::Arc;
use mv_core::*;
use mv_storage::unified::UnifiedStore;

/// The IntentEngine analyzes knowledge nodes to suggest autonomous actions.
#[derive(Clone)]
pub struct IntentEngine {
    store: Arc<UnifiedStore>,
}

impl IntentEngine {
    pub fn new(store: Arc<UnifiedStore>) -> Self {
        Self { store }
    }

    /// Extract possible intents from a node and store them.
    pub async fn extract_intents_and_store(&self, node: &KnowledgeNode) -> MvResult<Vec<CapturedIntent>> {
        let mut intents = Vec::new();

        if let Some(intent) = self.detect_reminder_intent(node) {
            intents.push(intent);
        }

        if let Some(intent) = self.detect_task_intent(node) {
            intents.push(intent);
        }

        intents.extend(self.detect_link_intents(node));
        intents.extend(self.detect_tag_intents(node));

        for intent in &intents {
            self.store.nodes.log_intent(intent).await?;
        }

        Ok(intents)
    }

    /// Detect reminder-related intents using string matching
    fn detect_reminder_intent(&self, node: &KnowledgeNode) -> Option<CapturedIntent> {
        let content_lower = node.content.to_lowercase();

        let is_reminder = content_lower.contains("remind me")
            || content_lower.contains("don't forget")
            || content_lower.contains("dont forget")
            || content_lower.contains("reminder:")
            || content_lower.contains("tomorrow")
            || content_lower.contains("next week")
            || content_lower.contains("next month");

        if !is_reminder {
            return None;
        }

        let mut intent = CapturedIntent::new(node.id, IntentType::ScheduleReminder);

        // Adjust confidence based on keyword strength
        if content_lower.contains("remind me") || content_lower.contains("don't forget") {
            intent.confidence = 0.9;
        } else if content_lower.contains("tomorrow") || content_lower.contains("next week") {
            intent.confidence = 0.75;
        } else {
            intent.confidence = 0.6;
        }

        // Extract date hints as parameters
        let mut params = serde_json::Map::new();
        if content_lower.contains("tomorrow") {
            params.insert("relative_time".into(), "tomorrow".into());
        } else if content_lower.contains("next week") {
            params.insert("relative_time".into(), "next_week".into());
        } else if content_lower.contains("next month") {
            params.insert("relative_time".into(), "next_month".into());
        }
        if !params.is_empty() {
            intent.parameters = serde_json::Value::Object(params);
        }

        Some(intent)
    }

    /// Detect task-related intents
    fn detect_task_intent(&self, node: &KnowledgeNode) -> Option<CapturedIntent> {
        let content = &node.content;
        let content_lower = content.to_lowercase();

        let is_task = content.contains("- [ ]")
            || content.contains("- []")
            || content_lower.contains("todo:")
            || content_lower.contains("task:")
            || content_lower.contains("need to ")
            || content_lower.contains("must ")
            || content_lower.contains("have to ")
            || content_lower.contains("action item");

        if !is_task {
            return None;
        }

        let mut intent = CapturedIntent::new(node.id, IntentType::ExtractTask);

        // Checkbox syntax is highest confidence
        if content.contains("- [ ]") || content.contains("- []") {
            intent.confidence = 0.95;
        } else if content_lower.contains("todo:") || content_lower.contains("task:") {
            intent.confidence = 0.85;
        } else if content_lower.contains("need to") || content_lower.contains("must") {
            intent.confidence = 0.7;
        } else {
            intent.confidence = 0.6;
        }

        Some(intent)
    }

    /// Detect wikilink and mention-style link intents
    fn detect_link_intents(&self, node: &KnowledgeNode) -> Vec<CapturedIntent> {
        let mut intents = Vec::new();

        // Detect [[wikilinks]] - simple bracket matching
        let mut i = 0;
        let chars: Vec<char> = node.content.chars().collect();
        while i < chars.len().saturating_sub(3) {
            if chars[i] == '[' && chars.get(i + 1) == Some(&'[') {
                // Found opening [[
                let start = i + 2;
                let mut end = start;
                while end < chars.len().saturating_sub(1) {
                    if chars[end] == ']' && chars.get(end + 1) == Some(&']') {
                        // Found closing ]]
                        let target: String = chars[start..end].iter().collect();
                        if !target.is_empty() {
                            let mut intent = CapturedIntent::new(node.id, IntentType::SuggestLink)
                                .with_confidence(0.85);

                            let mut params = serde_json::Map::new();
                            params.insert("target".into(), target.into());
                            params.insert("link_type".into(), "wikilink".into());
                            intent.parameters = serde_json::Value::Object(params);

                            intents.push(intent);
                        }
                        i = end + 2;
                        break;
                    }
                    end += 1;
                }
                if end >= chars.len().saturating_sub(1) {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Detect @mentions - simple word boundary matching
        let words: Vec<&str> = node.content.split_whitespace().collect();
        for word in words {
            if word.starts_with('@') && word.len() > 1 {
                let mention = word[1..].trim_matches(|c: char| !c.is_alphanumeric());
                if !mention.is_empty() {
                    let mut intent = CapturedIntent::new(node.id, IntentType::LinkToProject)
                        .with_confidence(0.75);

                    let mut params = serde_json::Map::new();
                    params.insert("target".into(), mention.to_string().into());
                    params.insert("link_type".into(), "mention".into());
                    intent.parameters = serde_json::Value::Object(params);

                    intents.push(intent);
                }
            }
        }

        intents
    }

    /// Detect potential tag suggestions based on hashtags in content
    fn detect_tag_intents(&self, node: &KnowledgeNode) -> Vec<CapturedIntent> {
        let mut intents = Vec::new();

        // Find #hashtags in content that aren't already tags
        let words: Vec<&str> = node.content.split_whitespace().collect();
        for word in words {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word[1..].trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
                if tag.is_empty() {
                    continue;
                }

                // Skip if already tagged
                if node.tags.iter().any(|t| t.to_lowercase() == tag) {
                    continue;
                }

                let mut intent = CapturedIntent::new(node.id, IntentType::SuggestTag)
                    .with_confidence(0.8);

                let mut params = serde_json::Map::new();
                params.insert("tag".into(), tag.into());
                intent.parameters = serde_json::Value::Object(params);

                intents.push(intent);
            }
        }

        intents
    }

    /// Analyze a single node for intents without storing
    pub fn analyze_node(&self, node: &KnowledgeNode) -> Vec<CapturedIntent> {
        let mut intents = Vec::new();

        if let Some(intent) = self.detect_reminder_intent(node) {
            intents.push(intent);
        }
        if let Some(intent) = self.detect_task_intent(node) {
            intents.push(intent);
        }
        intents.extend(self.detect_link_intents(node));
        intents.extend(self.detect_tag_intents(node));

        intents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(content: &str) -> KnowledgeNode {
        KnowledgeNode::new(NodeKind::Fact, content.to_string())
    }

    #[test]
    fn test_reminder_detection() {
        let store = Arc::new(mv_storage::unified::UnifiedStore::in_memory(384).unwrap());
        let engine = IntentEngine::new(store);

        let node = make_node("Remind me to call John tomorrow");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::ScheduleReminder)));

        let node = make_node("Don't forget to buy groceries");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::ScheduleReminder)));
    }

    #[test]
    fn test_task_detection() {
        let store = Arc::new(mv_storage::unified::UnifiedStore::in_memory(384).unwrap());
        let engine = IntentEngine::new(store);

        let node = make_node("- [ ] Complete the report");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::ExtractTask)));

        let node = make_node("TODO: Fix the bug in login");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::ExtractTask)));
    }

    #[test]
    fn test_link_detection() {
        let store = Arc::new(mv_storage::unified::UnifiedStore::in_memory(384).unwrap());
        let engine = IntentEngine::new(store);

        let node = make_node("See [[Project Alpha]] for details");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::SuggestLink)));

        let node = make_node("CC @john about this");
        let intents = engine.analyze_node(&node);
        assert!(intents.iter().any(|i| matches!(i.intent_type, IntentType::LinkToProject)));
    }

    #[test]
    fn test_tag_detection() {
        let store = Arc::new(mv_storage::unified::UnifiedStore::in_memory(384).unwrap());
        let engine = IntentEngine::new(store);

        let node = make_node("This is about #rust and #performance");
        let intents = engine.analyze_node(&node);
        assert_eq!(intents.iter().filter(|i| matches!(i.intent_type, IntentType::SuggestTag)).count(), 2);
    }
}
