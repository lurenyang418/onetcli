use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use gpui::Global;

use super::connector::{LlmConnector, LlmProvider};
use super::types::{ProviderConfig, ProviderType};

pub struct ProviderManager {
    providers: Arc<DashMap<i64, Arc<dyn LlmProvider>>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(DashMap::new()),
        }
    }

    pub async fn get_provider(&self, config: &ProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        let id = config.id;

        if let Some(provider) = self.providers.get(&id) {
            return Ok(Arc::clone(&*provider));
        }

        if !config.enabled {
            anyhow::bail!("Provider is disabled: {}", id);
        }

        let provider: Arc<dyn LlmProvider> = match config.provider_type {
            ProviderType::Pig => {
                anyhow::bail!("Pig provider is not supported")
            }
            _ => {
                let connector = LlmConnector::from_config(config)?;
                Arc::new(connector)
            }
        };

        self.providers.insert(id, Arc::clone(&provider));

        Ok(provider)
    }

    pub fn remove_provider(&self, id: i64) {
        self.providers.remove(&id);
    }

    pub fn clear_cache(&self) {
        self.providers.clear();
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GlobalProviderState {
    manager: Arc<ProviderManager>,
}

impl Clone for GlobalProviderState {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
        }
    }
}

impl GlobalProviderState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(ProviderManager::new()),
        }
    }

    pub fn manager(&self) -> Arc<ProviderManager> {
        Arc::clone(&self.manager)
    }
}

impl Default for GlobalProviderState {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for GlobalProviderState {}
