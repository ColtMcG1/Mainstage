use crate::vm::value::Value;
use async_trait::async_trait;

#[async_trait]
pub trait Plugin: Send + Sync {
    /// Name of the plugin (e.g. "cpp_compiler").
    fn name(&self) -> &str;

    /// Called by the runtime to invoke a function.
    /// `func` is the function name (e.g. "compile"), `args` are VM values.
    /// Return a VM `Value` for the runtime to use.
    async fn call(&self, func: &str, args: Vec<Value>) -> Result<Value, String>;

    /// Optional metadata for capabilities, versioning, etc.
    fn metadata(&self) -> PluginMetadata { PluginMetadata::default() }
}

#[derive(Default)]
pub struct PluginMetadata {
    pub description: String,
    pub version: String,
    pub arguments: Vec<String>,
    pub returns: Vec<String>,
}

use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::vm::manifest::PluginManifest;
use crate::vm::external::ExternalPlugin;

#[derive(Clone, Debug)]
pub struct PluginDescriptor {
    pub manifest: PluginManifest,
    pub path: Option<PathBuf>,
}

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    descriptors: HashMap<String, PluginDescriptor>,
}

impl PluginRegistry {
    pub fn new() -> Self { Self { plugins: HashMap::new(), descriptors: HashMap::new() } }

    /// Register a runtime plugin instance (in-process adapter).
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        self.plugins.insert(plugin.name().to_string(), plugin);
    }

    /// Register a manifest/descriptor without an instance (discovered on disk).
    pub fn register_descriptor(&mut self, manifest: PluginManifest, path: Option<PathBuf>) {
        let name = manifest.name.clone();
        self.descriptors.insert(name.clone(), PluginDescriptor { manifest, path });
    }

    /// If a descriptor points to an external executable, create and register an ExternalPlugin.
    pub fn try_register_external(&mut self, desc: &PluginDescriptor) {
        // Only register if descriptor has a path (directory of manifest)
        if let Some(dir) = &desc.path {
            let entry = desc.manifest.entry.clone().unwrap_or_else(|| desc.manifest.name.clone());
            let mut exe = dir.clone();
            exe.push(entry);
            // On Windows, allow .exe suffix if not provided
            if !exe.exists() {
                let mut exe_exe = exe.clone();
                exe_exe.set_extension("exe");
                if exe_exe.exists() {
                    exe = exe_exe;
                }
            }
            if exe.exists() {
                let ep = ExternalPlugin::new(desc.manifest.name.clone(), exe);
                self.register(std::sync::Arc::new(ep));
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(name).cloned()
    }

    pub fn get_descriptor(&self, name: &str) -> Option<&PluginDescriptor> {
        self.descriptors.get(name)
    }

    /// Return a cloned map of all registered descriptors for read-only consumers.
    pub fn descriptors_map(&self) -> std::collections::HashMap<String, PluginDescriptor> {
        self.descriptors.clone()
    }

    /// Return the list of currently registered runtime plugin names.
    pub fn registered_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub fn list_functions(&self, plugin_name: &str) -> Option<Vec<String>> {
        self.descriptors.get(plugin_name).map(|d| d.manifest.functions.iter().map(|f| f.name.clone()).collect())
    }

    pub fn unregister(&mut self, name: &str) {
        self.plugins.remove(name);
    }
}