use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressEnv {
    pub infra_ip: String,
    pub root_ip: String,
    pub chain_id: u64,
    pub rpc_http: String,
    pub deployer_api_port: u16,
    pub deployer_ui_port: u16,
}

impl Default for PressEnv {
    fn default() -> Self {
        Self {
            infra_ip: "38.146.25.37".to_string(),
            root_ip: "38.146.25.78".to_string(),
            chain_id: 9495,
            rpc_http: "http://press-rpc:8545".to_string(),
            deployer_api_port: 8085,
            deployer_ui_port: 8090,
        }
    }
}
