// Dependency resolution for service manager
use super::{ServiceInfo, ServiceConfig, ServiceStatus};
use anyhow::{Result, bail};
use std::collections::{HashSet, HashMap, VecDeque};

pub struct DependencyResolver;

impl DependencyResolver {
    /// Check if dependencies are satisfied for a service
    pub fn check_dependencies(
        services: &HashMap<String, ServiceInfo>,
        config: &ServiceConfig,
    ) -> Result<bool> {
        for dep in &config.dependencies {
            match services.get(dep) {
                Some(info) if info.status == ServiceStatus::Running => continue,
                Some(info) => {
                    log::debug!("Dependency {} is in state {:?}", dep, info.status);
                    return Ok(false);
                }
                None => {
                    log::warn!("Dependency {} not found", dep);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Detect circular dependencies in service configurations
    pub fn detect_circular_dependencies(
        configs: &HashMap<String, ServiceConfig>,
    ) -> Result<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for name in configs.keys() {
            if !visited.contains(name) {
                if Self::has_cycle(name, configs, &mut visited, &mut rec_stack)? {
                    bail!("Circular dependency detected involving service: {}", name);
                }
            }
        }

        Ok(())
    }

    fn has_cycle(
        service: &str,
        configs: &HashMap<String, ServiceConfig>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<bool> {
        visited.insert(service.to_string());
        rec_stack.insert(service.to_string());

        if let Some(config) = configs.get(service) {
            for dep in &config.dependencies {
                if !visited.contains(dep) {
                    if Self::has_cycle(dep, configs, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(dep) {
                    log::error!("Circular dependency: {} -> {}", service, dep);
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(service);
        Ok(false)
    }

    /// Calculate startup order using topological sort
    pub fn calculate_startup_order(
        configs: &HashMap<String, ServiceConfig>,
    ) -> Result<Vec<String>> {
        // First check for circular dependencies
        Self::detect_circular_dependencies(configs)?;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize structures
        for (name, config) in configs {
            in_degree.entry(name.clone()).or_insert(0);
            adj_list.entry(name.clone()).or_default();

            for dep in &config.dependencies {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
                adj_list.entry(name.clone()).or_default().push(dep.clone());
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(service) = queue.pop_front() {
            result.push(service.clone());

            if let Some(deps) = adj_list.get(&service) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        // Reverse to get correct startup order (dependencies first)
        result.reverse();

        if result.len() != configs.len() {
            bail!("Failed to resolve all dependencies - possible circular reference");
        }

        Ok(result)
    }

    /// Get all dependencies for a service (recursive)
    pub fn get_all_dependencies(
        service: &str,
        configs: &HashMap<String, ServiceConfig>,
    ) -> Result<HashSet<String>> {
        let mut all_deps = HashSet::new();
        let mut to_process = vec![service.to_string()];
        let mut processed = HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&current) {
                continue;
            }
            processed.insert(current.clone());

            if let Some(config) = configs.get(&current) {
                for dep in &config.dependencies {
                    all_deps.insert(dep.clone());
                    to_process.push(dep.clone());
                }
            }
        }

        Ok(all_deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{ServiceDefinition, RestartPolicy};

    #[test]
    fn test_circular_dependency_detection() {
        let mut configs = HashMap::new();
        
        // Create circular dependency: A -> B -> C -> A
        configs.insert("A".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "A".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec!["B".to_string()],
        });

        configs.insert("B".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "B".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec!["C".to_string()],
        });

        configs.insert("C".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "C".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec!["A".to_string()],
        });

        assert!(DependencyResolver::detect_circular_dependencies(&configs).is_err());
    }

    #[test]
    fn test_startup_order() {
        let mut configs = HashMap::new();
        
        // Create dependency chain: A depends on B, B depends on C
        configs.insert("A".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "A".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec!["B".to_string()],
        });

        configs.insert("B".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "B".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec!["C".to_string()],
        });

        configs.insert("C".to_string(), ServiceConfig {
            service: ServiceDefinition {
                name: "C".to_string(),
                command: "echo".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::Never,
                restart_delay_ms: 0,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![],
            dependencies: vec![],
        });

        let order = DependencyResolver::calculate_startup_order(&configs).unwrap();
        assert_eq!(order, vec!["C", "B", "A"]);
    }
}