use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ModuleGraph {
    pub required: HashSet<String>,
    pub deps: HashMap<String, Vec<String>>,
}

impl ModuleGraph {
    pub fn default_graph() -> Self {
        let mut required = HashSet::new();
        required.insert("core_chain".to_string());
        required.insert("press_token".to_string());
        required.insert("proposal_center".to_string());
        required.insert("press_council".to_string());
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        deps.insert("press_token".into(), vec!["core_chain".into()]);
        deps.insert("proposal_center".into(), vec!["core_chain".into(), "press_token".into()]);
        deps.insert("press_council".into(), vec!["proposal_center".into(), "press_token".into()]);
        deps.insert("press_court".into(), vec!["press_council".into(), "press_token".into()]);
        deps.insert("sync_plugin".into(), vec!["press_token".into(), "proposal_center".into()]);
        deps.insert("invisible_chain_publishing".into(), vec!["sync_plugin".into(), "press_token".into()]);
        deps.insert("source_secrecy_vault".into(), vec!["press_token".into()]);
        deps.insert("outlet_tokens".into(), vec!["press_token".into(), "proposal_center".into()]);
        deps.insert("liquidity_routing".into(), vec!["press_token".into(), "outlet_tokens".into()]);
        deps.insert("legacy_migration".into(), vec!["press_token".into()]);
        deps.insert("opinions".into(), vec!["press_token".into()]);
        deps.insert("ai_fact_dispute".into(), vec!["press_token".into(), "proposal_center".into()]);
        deps.insert("dispute_bonds".into(), vec!["press_token".into(), "press_court".into()]);
        deps.insert("earnings_vault".into(), vec!["press_token".into()]);
        deps.insert("licensing_engine".into(), vec!["press_token".into()]);
        deps.insert("ai_verification_api".into(), vec!["press_token".into()]);
        deps.insert("treasury_flywheel".into(), vec!["press_token".into()]);
        Self { required, deps }
    }

    pub fn normalize(&self, requested: &HashMap<String, bool>) -> HashMap<String, bool> {
        let mut enabled: HashMap<String, bool> = requested.clone();
        for r in self.required.iter() {
            enabled.insert(r.clone(), true);
        }
        let mut changed = true;
        while changed {
            changed = false;
            let keys: Vec<String> = enabled.keys().cloned().collect();
            for k in keys {
                if *enabled.get(&k).unwrap_or(&false) {
                    if let Some(ds) = self.deps.get(&k) {
                        for d in ds {
                            if !*enabled.get(d).unwrap_or(&false) {
                                enabled.insert(d.clone(), true);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        enabled
    }
}
