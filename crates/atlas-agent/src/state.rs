//! State management for agents

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

use atlas_core::Metadata;

use crate::error::Error;
use crate::{State, TaskState};

/// Memory entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Entry ID
    pub id: Uuid,
    
    /// Entry timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Entry data
    pub data: Value,
    
    /// Entry metadata
    pub metadata: Metadata,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(data: Value, metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            data,
            metadata,
        }
    }
}

/// Memory configuration
#[derive(Clone, Debug, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of entries to keep
    pub capacity: usize,
    
    /// Whether to persist memory to disk
    pub persistent: bool,
    
    /// Path to persist memory to
    pub persist_path: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            persistent: false,
            persist_path: None,
        }
    }
}

/// Agent state manager
#[derive(Debug)]
pub struct AgentStateManager {
    /// Agent state
    state: Arc<RwLock<State>>,
    
    /// Memory configuration
    memory_config: MemoryConfig,
    
    /// Memory entries
    memory: Arc<RwLock<Vec<MemoryEntry>>>,
}

impl AgentStateManager {
    /// Create a new state manager
    pub fn new(state: State, config: MemoryConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
            memory_config: config,
            memory: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the current state
    pub fn state(&self) -> &Arc<RwLock<State>> {
        &self.state
    }

    /// Update the state
    pub async fn update_state(&self, data: Metadata) -> Result<()> {
        let mut state = self.state.write().await;
        state.update(data)
    }

    /// Get a snapshot of the current state
    pub async fn snapshot(&self) -> Result<Metadata> {
        let state = self.state.read().await;
        state.snapshot()
    }

    /// Add a memory entry
    pub async fn add_memory(&self, data: Value, metadata: Metadata) -> Result<Uuid> {
        let entry = MemoryEntry::new(data, metadata);
        let id = entry.id;
        
        let mut memory = self.memory.write().await;
        
        // Enforce capacity limit
        if memory.len() >= self.memory_config.capacity {
            memory.remove(0);
        }
        
        memory.push(entry);
        
        // Persist if configured
        if self.memory_config.persistent {
            self.persist_memory().await?;
        }
        
        Ok(id)
    }

    /// Get a memory entry by ID
    pub async fn get_memory(&self, id: Uuid) -> Result<Option<MemoryEntry>> {
        let memory = self.memory.read().await;
        Ok(memory.iter().find(|e| e.id == id).cloned())
    }

    /// Search memory entries
    pub async fn search_memory(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let memory = self.memory.read().await;
        
        // Simple substring search for now
        // TODO: Implement proper search functionality
        Ok(memory
            .iter()
            .filter(|e| {
                serde_json::to_string(&e.data)
                    .unwrap_or_default()
                    .contains(query)
            })
            .cloned()
            .collect())
    }

    /// Get all memory entries
    pub async fn list_memory(&self) -> Result<Vec<MemoryEntry>> {
        let memory = self.memory.read().await;
        Ok(memory.clone())
    }

    /// Clear all memory entries
    pub async fn clear_memory(&self) -> Result<()> {
        let mut memory = self.memory.write().await;
        memory.clear();
        
        if self.memory_config.persistent {
            self.persist_memory().await?;
        }
        
        Ok(())
    }

    /// Get a task state by ID
    pub async fn get_task(&self, id: Uuid) -> Result<Option<TaskState>> {
        let state = self.state.read().await;
        Ok(state.tasks.get(&id).cloned())
    }

    /// Update a task state
    pub async fn update_task(&self, task: TaskState) -> Result<()> {
        let mut state = self.state.write().await;
        state.tasks.insert(task.id, task);
        Ok(())
    }

    /// Remove a task state
    pub async fn remove_task(&self, id: Uuid) -> Result<()> {
        let mut state = self.state.write().await;
        state.tasks.remove(&id);
        Ok(())
    }

    /// Persist memory to disk
    async fn persist_memory(&self) -> Result<()> {
        if let Some(path) = &self.memory_config.persist_path {
            let memory = self.memory.read().await;
            let json = serde_json::to_string_pretty(&*memory)?;
            tokio::fs::write(path, json).await?;
        }
        Ok(())
    }

    /// Load memory from disk
    async fn load_memory(&self) -> Result<()> {
        if let Some(path) = &self.memory_config.persist_path {
            if tokio::fs::try_exists(path).await? {
                let json = tokio::fs::read_to_string(path).await?;
                let entries: Vec<MemoryEntry> = serde_json::from_str(&json)?;
                
                let mut memory = self.memory.write().await;
                *memory = entries;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_state_update() {
        let manager = AgentStateManager::new(State::default(), MemoryConfig::default());
        
        let mut data = Metadata::new();
        data.insert("test", "value");
        
        manager.update_state(data).await.unwrap();
        
        let state = manager.state.read().await;
        assert_eq!(
            state.memory.get("test").unwrap().as_str().unwrap(),
            "value"
        );
    }

    #[tokio::test]
    async fn test_memory_management() {
        let manager = AgentStateManager::new(
            State::default(),
            MemoryConfig {
                capacity: 2,
                ..Default::default()
            },
        );

        let id1 = manager
            .add_memory(json!("entry1"), Metadata::new())
            .await
            .unwrap();
        let id2 = manager
            .add_memory(json!("entry2"), Metadata::new())
            .await
            .unwrap();
        let id3 = manager
            .add_memory(json!("entry3"), Metadata::new())
            .await
            .unwrap();

        // First entry should be removed due to capacity limit
        assert!(manager.get_memory(id1).await.unwrap().is_none());
        assert!(manager.get_memory(id2).await.unwrap().is_some());
        assert!(manager.get_memory(id3).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_memory_search() {
        let manager = AgentStateManager::new(State::default(), MemoryConfig::default());

        manager
            .add_memory(json!({"text": "test entry"}), Metadata::new())
            .await
            .unwrap();
        manager
            .add_memory(json!({"text": "another entry"}), Metadata::new())
            .await
            .unwrap();

        let results = manager.search_memory("test").await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
