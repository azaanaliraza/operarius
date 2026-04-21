use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderWorkflow {
    pub id: Option<String>,
    pub name: String,
    pub nodes: Vec<BuilderNode>,
    pub edges: Vec<BuilderEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderNode {
    pub id: String,
    #[serde(default)]
    pub node_type: Option<String>,
    pub data: BuilderNodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderNodeData {
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderEdge {
    pub id: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompiledWorkflow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub execution_order: Vec<String>,
    pub warnings: Vec<String>,
    pub capability_flags: Vec<String>,
    pub manifest: Value,
}

pub fn compile(workflow: &BuilderWorkflow) -> Result<CompiledWorkflow, String> {
    if workflow.nodes.is_empty() {
        return Err("Workflow has no nodes".to_string());
    }

    let nodes_by_id: HashMap<String, &BuilderNode> = workflow
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect();

    let mut indegree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for node in &workflow.nodes {
        indegree.insert(node.id.clone(), 0);
    }

    for edge in &workflow.edges {
        if !nodes_by_id.contains_key(&edge.source) || !nodes_by_id.contains_key(&edge.target) {
            return Err(format!("Invalid edge {} references missing node", edge.id));
        }

        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());

        *indegree.entry(edge.target.clone()).or_insert(0) += 1;
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter_map(|(id, deg)| if *deg == 0 { Some(id.clone()) } else { None })
        .collect();

    let mut execution_order = Vec::new();
    while let Some(id) = queue.pop_front() {
        execution_order.push(id.clone());
        if let Some(neighbors) = adjacency.get(&id) {
            for next in neighbors {
                if let Some(deg) = indegree.get_mut(next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
    }

    if execution_order.len() != workflow.nodes.len() {
        return Err("Workflow graph contains a cycle".to_string());
    }

    let mut warnings = Vec::new();

    let has_agent = workflow.nodes.iter().any(|n| n.data.kind == "agent");
    if !has_agent {
        warnings.push("No agent node found. Add a Hermes Agent node as the orchestrator.".to_string());
    }

    let has_trigger = workflow.nodes.iter().any(|n| n.data.kind == "trigger");
    if !has_trigger {
        warnings.push("No trigger node found. Add at least one entry trigger.".to_string());
    }

    let has_model = workflow.nodes.iter().any(|n| n.data.kind == "model");
    if !has_model {
        warnings.push("No model node found. Add a Local/Remote LLM model node.".to_string());
    }

    let has_memory = workflow.nodes.iter().any(|n| n.data.kind == "memory");
    if !has_memory {
        warnings.push("No memory node found. Add memory to preserve context across tasks.".to_string());
    }

    let tool_nodes = workflow
        .nodes
        .iter()
        .filter(|n| n.data.kind == "tool")
        .collect::<Vec<&BuilderNode>>();

    if tool_nodes.is_empty() {
        warnings.push("No tool node found. Add tools for actions such as Slack, Jira, email, and analytics.".to_string());
    }

    let mut capability_flags = Vec::new();
    let lowers = workflow
        .nodes
        .iter()
        .map(|n| {
            format!(
                "{} {}",
                n.data.label.to_lowercase(),
                serde_json::to_string(&n.data.config)
                    .unwrap_or_default()
                    .to_lowercase()
            )
        })
        .collect::<Vec<String>>();

    let has_keyword = |k: &str| lowers.iter().any(|s| s.contains(k));

    if has_keyword("gmail") || has_keyword("email") || has_keyword("inbox") {
        capability_flags.push("email_triage".to_string());
    }
    if has_keyword("slides") || has_keyword("deck") || has_keyword("town hall") {
        capability_flags.push("deck_review".to_string());
    }
    if has_keyword("jira") {
        capability_flags.push("jira_ops".to_string());
    }
    if has_keyword("ga4") || has_keyword("gsc") || has_keyword("google ads") || has_keyword("seo") {
        capability_flags.push("marketing_analytics".to_string());
    }
    if has_keyword("slack") {
        capability_flags.push("slack_webhook".to_string());
    }
    if has_keyword("rag") || has_keyword("knowledge") || has_keyword("zendesk") {
        capability_flags.push("support_automation".to_string());
    }
    if has_keyword("codex") || has_keyword("code") || has_keyword("cli") {
        capability_flags.push("developer_automation".to_string());
    }

    if capability_flags.is_empty() {
        capability_flags.push("general_agent_automation".to_string());
    }

    let workflow_id = workflow
        .id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let manifest = serde_json::json!({
        "version": "1.0",
        "workflow_id": workflow_id,
        "name": workflow.name,
        "execution_order": execution_order,
        "nodes": workflow.nodes,
        "edges": workflow.edges,
        "capability_flags": capability_flags,
    });

    Ok(CompiledWorkflow {
        workflow_id,
        workflow_name: workflow.name.clone(),
        node_count: workflow.nodes.len(),
        edge_count: workflow.edges.len(),
        execution_order: manifest["execution_order"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect(),
        warnings,
        capability_flags: manifest["capability_flags"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect(),
        manifest,
    })
}
