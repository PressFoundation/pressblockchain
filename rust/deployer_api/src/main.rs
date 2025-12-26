use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use std::process::Command;

fn sh(step: &str, engine: &Engine, cmd: &str) -> Result<String, String> {
    engine.write_log(step, &format!("$ {}", cmd));
    let out = Command::new("sh").arg("-lc").arg(cmd).output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !stdout.is_empty() { engine.write_log(step, &stdout); }
    if !stderr.is_empty() { engine.write_log(step, &stderr); }
    if out.status.success() { Ok(stdout) } else { Err(format!("cmd failed: {}", cmd)) }
}

fn ensure_deployer_key(engine: &Engine, step: &str) -> Result<String, String> {
    let key_path = engine.state_dir.join("deployer.privatekey");
    if key_path.exists() {
        return Ok(fs::read_to_string(key_path).map_err(|e| e.to_string())?.trim().to_string());
    }
    let key_bytes = {
        let mut b = [0u8; 32];
        getrandom::getrandom(&mut b).map_err(|e| e.to_string())?;
        b
    };
    let hex = format!("0x{}", hex::encode(key_bytes));
    fs::write(&key_path, format!("{}\n", hex)).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).ok();
    }
    engine.write_log(step, "Generated state/deployer.privatekey (chmod 600).");
    Ok(hex)
}

fn fund_deployer(engine: &Engine, step: &str, rpc: &str, addr: &str) -> Result<(), String> {
    // anvil_setBalance supports hex balance
    let bal = "0x3635C9ADC5DEA00000"; // 1000 ETH
    let payload = format!(
        '{{"jsonrpc":"2.0","id":1,"method":"anvil_setBalance","params":["{}","{}"]}}',
        addr, bal
    );
    let cmd = format!("curl -sS -H 'Content-Type: application/json' --data '{}' {}", payload, rpc);
    let _ = sh(step, engine, &cmd)?;
    Ok(())
}


#[derive(Clone)]
struct AppState {
    state_dir: PathBuf,
    engine: Arc<Mutex<Engine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StepStatus {
    Pending,
    Running,
    Success,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Step {
    id: String,
    name: String,
    status: StepStatus,
    started_at: Option<u64>,
    ended_at: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunState {
    run_id: String,
    created_at: u64,
    updated_at: u64,
    clean_start: bool,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct RunReq {
    clean_start: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ConfigRequest {
    openai_api_key: String,
    // Core installer parameter seeding (written to /state/press.env)
    chain_id: Option<u64>,
    infra_ip: Option<String>,
    root_ip: Option<String>,
    rpc_url: Option<String>,
    treasury_address: Option<String>,
    council_multisig_address: Option<String>,

    // Article approvals
    article_vote_window_seconds: Option<u64>,
    article_community_approvals_min: Option<u64>,
    article_outlet_approvals_min: Option<u64>,
    article_council_approvals_min: Option<u64>,
    article_flags_max: Option<u64>,
    article_vote_fee_community_press_wei: Option<String>,
    article_vote_fee_outlet_press_wei: Option<String>,
    article_vote_fee_council_press_wei: Option<String>,

    // Proposal governance
    proposal_min_total_votes: Option<u64>,
    proposal_yes_bps: Option<u64>,
    proposal_min_total_votes_major: Option<u64>,
    proposal_yes_bps_major: Option<u64>,
    proposal_duration_seconds: Option<u64>,
    proposal_max_duration_seconds: Option<u64>,
    proposal_vote_fee_press_wei: Option<String>,
    proposal_vote_fee_major_press_wei: Option<String>,
    proposal_vote_fee_grant_press_wei: Option<String>,
    proposal_vote_fee_court_press_wei: Option<String>,
    proposal_execute_min_total_votes: Option<u64>,
    proposal_execute_yes_bps: Option<u64>,

    // Treasury fee
    treasury_fee_bps: Option<u64>,
}


#[derive(Debug)]
struct Engine {
    state_dir: PathBuf,
}

impl Engine {
    fn new(state_dir: PathBuf) -> Self {
        fs::create_dir_all(state_dir.join("logs")).ok();
        Self { state_dir }
    }

    fn now() -> u64 {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap())
            .as_secs()
    }

    fn state_path(&self) -> PathBuf {
        self.state_dir.join("runtime.status.json")
    }

    fn log_path(&self, step: &str) -> PathBuf {
        self.state_dir.join("logs").join(format!("{step}.log"))
    }

    fn write_log(&self, step: &str, line: &str) {
        let p = self.log_path(step);
        let mut s = if p.exists() {
            fs::read_to_string(&p).unwrap_or_default()
        } else {
            String::new()
        };
        s.push_str(line);
        if !line.ends_with('\n') {
            s.push('\n');
        }
        fs::write(p, s).ok();
    }

    fn clear_log(&self, step: &str) {
        fs::write(self.log_path(step), "").ok();
    }

    fn default_steps(clean_start: bool) -> Vec<Step> {
        vec![
            Step {
                id: "preflight".into(),
                name: "Preflight (disk/docker/ports)".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "clean_start".into(),
                name: "Clean Start (optional)".into(),
                status: if clean_start {
                    StepStatus::Pending
                } else {
                    StepStatus::Skipped
                },
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "rpc_up".into(),
                name: "RPC Up (Anvil)".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "press_deploy".into(),
                name: "Deploy PRESS + core contracts".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "deploy_exchange".into(),
                name: "Deploy Exchange Listing Registry".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "seed_fees".into(),
                name: "Seed listing fees (Basic/Pro/Elite)".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "rotate_admin_token".into(),
                name: "Rotate admin token (feature controls)".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
            Step {
                id: "verify".into(),
                name: "Verify endpoints".into(),
                status: StepStatus::Pending,
                started_at: None,
                ended_at: None,
                error: None,
            },
        ]
    }

    fn new_run(&self, clean_start: bool) -> RunState {
        let run_id = format!("run_{}", Uuid::new_v4());
        let t = Self::now();
        let s = RunState {
            run_id,
            created_at: t,
            updated_at: t,
            clean_start,
            steps: Self::default_steps(clean_start),
        };
        self.write_state(&s);
        s
    }

    fn read_state(&self) -> RunState {
        let p = self.state_path();
        if !p.exists() {
            return self.new_run(false);
        }
        serde_json::from_str(&fs::read_to_string(p).unwrap_or_else(|_| "{}".into()))
            .unwrap_or_else(|_| self.new_run(false))
    }

    fn write_state(&self, s: &RunState) {
        fs::write(self.state_path(), serde_json::to_string_pretty(s).unwrap()).ok();
    }

    fn start_step(&self, s: &mut RunState, id: &str) {
        for st in &mut s.steps {
            if st.id == id {
                st.status = StepStatus::Running;
                st.started_at = Some(Self::now());
                st.error = None;
            }
        }
        s.updated_at = Self::now();
        self.write_state(s);
    }

    fn end_step(&self, s: &mut RunState, id: &str, status: StepStatus, err: Option<String>) {
        for st in &mut s.steps {
            if st.id == id {
                st.status = status;
                st.ended_at = Some(Self::now());
                st.error = err;
            }
        }
        s.updated_at = Self::now();
        self.write_state(s);
    }

    fn apply_fix(&self, fix: &str) {
        match fix {
            "safe_ports" => self.write_log(
                "preflight",
                "Fix(safe_ports): API 8085, UI 8090, RPC 8545. Never bind host :80/:443.",
            ),
            "clean_orphans" => self.write_log(
                "clean_start",
                "Fix(clean_orphans): docker compose down -v --remove-orphans; remove *-run-* containers.",
            ),
            "docker_perm_hint" => self.write_log(
                "preflight",
                "Fix(docker_perm_hint): ensure docker socket perms allow controller (root or docker group).",
            ),
"start_docker" => {
    let _ = Command::new("bash").arg("-lc").arg("systemctl start docker || true; systemctl enable docker || true").output();
    self.write_log("preflight", "Fix(start_docker): attempted to start/enable docker.");
}
"chmod_state" => {
    let _ = Command::new("bash").arg("-lc").arg("chmod -R 777 /state /repo/state 2>/dev/null || true").output();
    self.write_log("preflight", "Fix(chmod_state): ensured /state and /repo/state are writable.");
}
"recreate_network" => {
    let _ = Command::new("bash").arg("-lc").arg("docker network create pressblockchain_default 2>/dev/null || true").output();
    self.write_log("preflight", "Fix(recreate_network): ensured pressblockchain_default exists.");
}
"compose_down" => {
    let _ = Command::new("bash").arg("-lc").arg("cd /repo && docker compose -f ops/docker/docker-compose.stack.yml down -v --remove-orphans || true").output();
    self.write_log("clean_start", "Fix(compose_down): compose down -v --remove-orphans executed.");
}
            _ => self.write_log("preflight", "Unknown fix id"),
        }
    }

    
async fn configure(State(st): State<AppState>, Json(req): Json<ConfigRequest>) -> Json<serde_json::Value> {
    // Store OpenAI key securely in /state/secrets.env (600 perms). Never log the key.
    if !req.openai_api_key.is_empty() && !req.openai_api_key.starts_with("sk-") {
        return Json(serde_json::json!({"ok": false, "error":"Invalid OpenAI key format"}));
    }
    let secrets_path = st.state_dir.join("secrets.env");
    let mut lines = String::new();
    if secrets_path.exists() {
        lines = fs::read_to_string(&secrets_path).unwrap_or_default();
        // remove old OPENAI_API_KEY line
        lines = lines.lines().filter(|l| !l.starts_with("OPENAI_API_KEY=")).map(|l| format!("{}\n", l)).collect();
    }
    if !req.openai_api_key.is_empty() {
        lines.push_str(&format!("OPENAI_API_KEY={}\n", req.openai_api_key));
    }
    fs::write(&secrets_path, lines).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600));
    }
    
        // Write installer parameters to /state/press.env so deploy step can seed all PressParameters deterministically.
        // Never write OPENAI key here (kept in secrets.env only).
        let env_path = st.state_dir.join("press.env");
        let mut env_lines = String::new();
        if env_path.exists() {
            env_lines = fs::read_to_string(&env_path).unwrap_or_default();
        }
        // helper to upsert KEY=VALUE
        fn upsert(mut s: String, key: &str, value: &str) -> String {
            let mut out = String::new();
            for line in s.lines() {
                if line.starts_with(&format!("{key}=")) { continue; }
                out.push_str(line);
                out.push('
');
            }
            out.push_str(&format!("{key}={value}
"));
            out
        }

        if let Some(v) = req.chain_id { env_lines = upsert(env_lines, "CHAIN_ID", &v.to_string()); }
        if let Some(v) = &req.infra_ip { env_lines = upsert(env_lines, "INFRA_IP", v); }
        if let Some(v) = &req.root_ip { env_lines = upsert(env_lines, "ROOT_IP", v); }
        if let Some(v) = &req.rpc_url { env_lines = upsert(env_lines, "PRESS_RPC_URL", v); }
        if let Some(v) = &req.treasury_address { env_lines = upsert(env_lines, "PRESS_TREASURY_ADDRESS", v); }
        if let Some(v) = &req.council_multisig_address { env_lines = upsert(env_lines, "COUNCIL_MULTISIG_ADDRESS", v); }

        // Article approvals
        if let Some(v) = req.article_vote_window_seconds { env_lines = upsert(env_lines, "ARTICLE_VOTE_WINDOW_SECONDS", &v.to_string()); }
        if let Some(v) = req.article_community_approvals_min { env_lines = upsert(env_lines, "ARTICLE_COMMUNITY_APPROVALS_MIN", &v.to_string()); }
        if let Some(v) = req.article_outlet_approvals_min { env_lines = upsert(env_lines, "ARTICLE_OUTLET_APPROVALS_MIN", &v.to_string()); }
        if let Some(v) = req.article_council_approvals_min { env_lines = upsert(env_lines, "ARTICLE_COUNCIL_APPROVALS_MIN", &v.to_string()); }
        if let Some(v) = req.article_flags_max { env_lines = upsert(env_lines, "ARTICLE_FLAGS_MAX", &v.to_string()); }
        if let Some(v) = &req.article_vote_fee_community_press_wei { env_lines = upsert(env_lines, "ARTICLE_VOTE_FEE_COMMUNITY_PRESS_WEI", v); }
        if let Some(v) = &req.article_vote_fee_outlet_press_wei { env_lines = upsert(env_lines, "ARTICLE_VOTE_FEE_OUTLET_PRESS_WEI", v); }
        if let Some(v) = &req.article_vote_fee_council_press_wei { env_lines = upsert(env_lines, "ARTICLE_VOTE_FEE_COUNCIL_PRESS_WEI", v); }

        // Proposals
        if let Some(v) = req.proposal_min_total_votes { env_lines = upsert(env_lines, "PROPOSAL_MIN_TOTAL_VOTES", &v.to_string()); }
        if let Some(v) = req.proposal_yes_bps { env_lines = upsert(env_lines, "PROPOSAL_YES_BPS", &v.to_string()); }
        if let Some(v) = req.proposal_min_total_votes_major { env_lines = upsert(env_lines, "PROPOSAL_MIN_TOTAL_VOTES_MAJOR", &v.to_string()); }
        if let Some(v) = req.proposal_yes_bps_major { env_lines = upsert(env_lines, "PROPOSAL_YES_BPS_MAJOR", &v.to_string()); }
        if let Some(v) = req.proposal_duration_seconds { env_lines = upsert(env_lines, "PROPOSAL_DURATION_SECONDS", &v.to_string()); }
        if let Some(v) = req.proposal_max_duration_seconds { env_lines = upsert(env_lines, "PROPOSAL_MAX_DURATION_SECONDS", &v.to_string()); }
        if let Some(v) = &req.proposal_vote_fee_press_wei { env_lines = upsert(env_lines, "PROPOSAL_VOTE_FEE_PRESS_WEI", v); }
        if let Some(v) = &req.proposal_vote_fee_major_press_wei { env_lines = upsert(env_lines, "PROPOSAL_VOTE_FEE_MAJOR_PRESS_WEI", v); }
        if let Some(v) = &req.proposal_vote_fee_grant_press_wei { env_lines = upsert(env_lines, "PROPOSAL_VOTE_FEE_GRANT_PRESS_WEI", v); }
        if let Some(v) = &req.proposal_vote_fee_court_press_wei { env_lines = upsert(env_lines, "PROPOSAL_VOTE_FEE_COURT_PRESS_WEI", v); }
        if let Some(v) = req.proposal_execute_min_total_votes { env_lines = upsert(env_lines, "PROPOSAL_EXECUTE_MIN_TOTAL_VOTES", &v.to_string()); }
        if let Some(v) = req.proposal_execute_yes_bps { env_lines = upsert(env_lines, "PROPOSAL_EXECUTE_YES_BPS", &v.to_string()); }

        if let Some(v) = req.treasury_fee_bps { env_lines = upsert(env_lines, "TREASURY_FEE_BPS", &v.to_string()); }

        if !env_lines.is_empty() {
            fs::write(&env_path, env_lines).ok();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&env_path, fs::Permissions::from_mode(0o600));
            }
        }
    Json(serde_json::json!({"ok": true, "openai_key_set": !req.openai_api_key.is_empty()}))
}

async fn fix_and_retry(State(st): State<AppState>) -> Json<serde_json::Value> {
    let eng = st.engine.lock().await;
    let rs = eng.read_state();
    let failed = rs.run_state.last_failed_step.clone();
    let err = rs.run_state.last_error.clone().unwrap_or_default();
    if failed.is_none() {
        return Json(serde_json::json!({"ok": false, "message":"No failed step recorded"}));
    }
    let fixes = rs.run_state.suggested_fixes.clone().unwrap_or_else(|| infer_fixes_from_error(&err));
    for fx in fixes.iter() { eng.apply_fix(fx); }
    // retry
    drop(eng);
    let eng2 = st.engine.lock().await;
    let mut s2 = eng2.read_state().run_state;
    let step = failed.unwrap();
    if let Err(e) = eng2.run_step(&mut s2, &step).await {
        return Json(serde_json::json!({"ok": false, "message": format!("Retry failed: {}", e), "fixes": fixes}));
    }
    return Json(serde_json::json!({"ok": true, "message":"Fix applied and step retried", "fixes": fixes}));
}

async fn run_all(&self, clean_start: bool, auto_fix: bool) -> Result<RunState, String> {
        let mut s = self.new_run(clean_start);
        let steps = s.steps.clone();
        for st in steps {
            if matches!(st.status, StepStatus::Skipped) {
                continue;
            }
            if let Err(e) = self.run_step(&mut s, &st.id).await {
    // record failure
    s.last_error = Some(e.clone());
    s.last_failed_step = Some(st.id.clone());
    let fixes = infer_fixes_from_error(&e);
    s.suggested_fixes = Some(fixes.clone());
    self.write_state(&s);
    if auto_fix {
        self.write_log(&st.id, "Auto-fix enabled: attempting known remediations…");
        for fx in fixes.iter() {
            self.apply_fix(fx);
        }
        self.write_log(&st.id, "Auto-fix applied. Retrying failed step once…");
        self.write_state(&s);
        // retry once
        if let Err(e2) = self.run_step(&mut s, &st.id).await {
            s.last_error = Some(e2);
            s.last_failed_step = Some(st.id.clone());
            self.write_state(&s);
            return Err("step failed after auto-fix".into());
        }
    } else {
        return Err("step failed".into());
    }
}

        }
        Ok(self.read_state())
    }

    

async fn fix_and_retry(State(st): State<AppState>) -> Json<serde_json::Value> {
    let eng = st.engine.lock().await;
    let rs = eng.read_state();
    let failed = rs.run_state.last_failed_step.clone();
    let err = rs.run_state.last_error.clone().unwrap_or_default();
    if failed.is_none() {
        return Json(serde_json::json!({"ok": false, "message":"No failed step recorded"}));
    }
    let fixes = rs.run_state.suggested_fixes.clone().unwrap_or_else(|| infer_fixes_from_error(&err));
    for fx in fixes.iter() { eng.apply_fix(fx); }
    // retry
    drop(eng);
    let eng2 = st.engine.lock().await;
    let mut s2 = eng2.read_state().run_state;
    let step = failed.unwrap();
    if let Err(e) = eng2.run_step(&mut s2, &step).await {
        return Json(serde_json::json!({"ok": false, "message": format!("Retry failed: {}", e), "fixes": fixes}));
    }
    return Json(serde_json::json!({"ok": true, "message":"Fix applied and step retried", "fixes": fixes}));
}

async fn run_step(&self, s: &mut RunState, id: &str) -> Result<(), String> {
        self.clear_log(id);
        self.start_step(s, id);

        match id {
            "preflight" => {
                let _ = ensure_proposal_presets(self, id);

                let _ = ensure_listing_tiers(self, id);
                let _ = ensure_treasury_key(self, id);

                sh(id, self, "df -h . | tail -n +2").ok();
                sh(id, self, "docker --version").ok();
                sh(id, self, "docker compose version").ok();
                sh(id, self, "ss -ltnp | head -n 120 || true").ok();
                self.end_step(s, id, StepStatus::Success, None);
                Ok(())
            }
            "clean_start" => {
                if !s.clean_start {
                    self.end_step(s, id, StepStatus::Skipped, None);
                    return Ok(());
                }
                sh(id, self, "cd /repo && docker compose -f ops/docker/docker-compose.stack.yml down -v --remove-orphans || true").ok();
                sh(id, self, "docker ps -a --format \"{{.Names}}\" | grep -E \"press-.*-run-\" | xargs -r docker rm -f || true").ok();
                sh(id, self, "docker network ls --format \"{{.Name}}\" | grep -E \"^press-\" | xargs -r docker network rm || true").ok();
                self.end_step(s, id, StepStatus::Success, None);
                Ok(())
            }
"stack_up" => {
    self.write_log(id, "Bringing up full stack via docker compose…");
    // Never bind host :80/:443; stack uses safe ports (RPC 8545 internal, Gateway 8085).
    let cmd = "cd /repo && docker compose -f ops/docker/docker-compose.stack.yml up -d --remove-orphans";
    sh(id, self, cmd)?;
    self.write_log(id, "Stack up complete.");
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
"rpc_up" => {
    self.write_log(id, "Verifying JSON-RPC health via eth_chainId…");
    let rpc_http = std::env::var("RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    let cmd = format!(
        "docker run --rm --network pressblockchain_default curlimages/curl:8.5.0 -sS -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_chainId\",\"params\":[]}' {}",
        rpc_http
    );
    let outp = sh(id, self, &cmd)?;
    if !outp.contains("result") {
        return Err(format!("RPC healthcheck failed: {}", outp));
    }
    self.write_log(id, &format!("RPC OK: {}", outp.trim()));
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
"press_deploy" => {
    // Generate deployer key (stored on host volume) then fund it on anvil,
    // then run Foundry deployment script in a container.
    let rpc_http = std::env::var("RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    let pk = ensure_deployer_key(self, id)?;
    // derive address using foundry cast inside container
    let addr = sh(id, self, &format!(
        "docker run --rm --network pressblockchain_default ghcr.io/foundry-rs/foundry:latest sh -lc "cast wallet address --private-key {}"",
        pk
    ))?.trim().to_string();

    self.write_log(id, &format!("Deployer address: {}", addr));
    fund_deployer(self, id, &rpc_http, &addr)?;

    // Run foundry deploy script
    let cmd = format!(
        "docker run --rm --network pressblockchain_default -v /repo:/repo -v /state:/state -w /repo/contracts \
          -e STATE_DIR=/state -e ETH_RPC_URL={} -e DEPLOYER_PRIVATE_KEY={} \
          ghcr.io/foundry-rs/foundry:latest sh -lc \
          "forge --version && (test -d lib/forge-std || forge install foundry-rs/forge-std --no-commit) && forge build && forge script script/Deploy.s.sol:Deploy --broadcast"",
        rpc_http, pk.replace("0x","")
    );
    sh(id, self, &cmd)?;

    
// Wire on-chain heartbeat defaults (enabled by default). Read deployed beacon address.
let ub_path = self.state_dir.join("uptime_beacon_address.txt");
if ub_path.exists() {
    let ub_addr = fs::read_to_string(&ub_path).unwrap_or_default().trim().to_string();
    let env_path = self.state_dir.join("press.env");
    let mut env_lines = fs::read_to_string(&env_path).unwrap_or_default();
    // helper upsert
    fn upsert_line(mut s: String, key: &str, value: &str) -> String {
        let mut out = String::new();
        for line in s.lines() {
            if line.starts_with(&format!("{key}=")) { continue; }
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(&format!("{key}={value}\n"));
        out
    }
    env_lines = upsert_line(env_lines, "PRESS_ONCHAIN_HEARTBEAT_ENABLED", "true");
    env_lines = upsert_line(env_lines, "PRESS_ONCHAIN_HEARTBEAT_INTERVAL_SEC", "300");
    env_lines = upsert_line(env_lines, "PRESS_ONCHAIN_HEARTBEAT_RPC", &rpc_http);
    env_lines = upsert_line(env_lines, "PRESS_ONCHAIN_HEARTBEAT_CONTRACT", &ub_addr);
    env_lines = upsert_line(env_lines, "PRESS_ONCHAIN_HEARTBEAT_PRIVKEY", &pk);
    fs::write(&env_path, env_lines).ok();
    self.write_log(id, &format!("On-chain heartbeat enabled by default (contract: {ub_addr})."));
} else {
    self.write_log(id, "Note: uptime_beacon_address.txt not found; heartbeat defaults not applied.");
}
self.write_log(id, "Deployment complete. Addresses written to state/deploy.json and token meta files.");
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
            "verify" => {
                sh(id, self, "curl -sS http://localhost:8085/health || true").ok();
                sh(id, self, "curl -sS -H \"Content-Type: application/json\" --data '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_chainId\",\"params\":[]}' http://press-rpc:8545 || true").ok();
                sh(id, self, "test -f /state/deploy.json && cat /state/deploy.json || true").ok();
                self.end_step(s, id, StepStatus::Success, None);
                Ok(())
            }
"keys" => {
    // Generate deployer key and admin token automatically so the operator never has to paste later.
    let _ = ensure_deployer_key(self, id)?;
    // Rotate admin token if not set
    let path = std::path::Path::new("/state/admin_token.txt");
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing.trim().is_empty() || existing.contains("CHANGE_ME") {
        let tok = format!("pb_admin_{}", hex::encode(rand_bytes(24)));
        std::fs::write(path, format!("{}
", tok)).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        self.write_log(id, "Admin token generated and stored to /state/admin_token.txt (not printed).");
    } else {
        self.write_log(id, "Admin token already set; leaving unchanged.");
    }
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
"deploy_exchange" => {
    self.write_log(id, "Deploying ExchangeListingRegistry (3 listing tiers)…");
    let rpc_http = std::env::var("RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    let pk = ensure_deployer_key(self, id)?;
    // run repo script which writes exchangeListingRegistry into state/deploy.json
    let cmd = format!(
        "cd /repo && RPC_URL={} DEPLOYER_KEY={} bash ops/scripts/deploy_exchange_listing.sh",
        rpc_http,
        pk
    );
    sh(id, self, &cmd)?;
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
"seed_fees" => {
    self.write_log(id, "Seeding listing fee parameters (Basic/Pro/Elite) via PressParameters.set…");
    let rpc_http = std::env::var("RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    // read pressParameters address from deploy.json
    let deploy = read_deploy_json().unwrap_or_default();
    let pp = deploy.get("pressParameters").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if pp.is_empty() {
        self.write_log(id, "pressParameters missing in state/deploy.json — skipping.");
        self.end_step(s, id, StepStatus::Skipped, None);
        return Ok(());
    }
    let pk = ensure_deployer_key(self, id)?;
    // Recommended defaults (tune later via governance):
    // Basic 2500 PRESS, Pro 8000 PRESS, Elite 25000 PRESS (18 decimals)
    let b = "2500000000000000000000";
    let p = "8000000000000000000000";
    let e = "25000000000000000000000";

    let cmd = format!(
        "docker run --rm --network pressblockchain_default -v /repo:/repo -w /repo ghcr.io/foundry-rs/foundry:latest sh -lc '\
            cast send --rpc-url \"{}\" --private-key \"{}\" \"{}\" \"set(bytes32,uint256)\" \"$(cast keccak \"listing_fee_basic\")\" {} && \
            cast send --rpc-url \"{}\" --private-key \"{}\" \"{}\" \"set(bytes32,uint256)\" \"$(cast keccak \"listing_fee_pro\")\" {} && \
            cast send --rpc-url \"{}\" --private-key \"{}\" \"{}\" \"set(bytes32,uint256)\" \"$(cast keccak \"listing_fee_elite\")\" {} \
        '",
        rpc_http, pk, pp, b,
        rpc_http, pk, pp, p,
        rpc_http, pk, pp, e
    );
    let _ = sh(id, self, &cmd).map(|o| self.write_log(id, &format!("Params set output:
{}", o)));
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
"rotate_admin_token" => {
    self.write_log(id, "Ensuring admin token is set (feature controls)…");
    let path = std::path::Path::new("/state/admin_token.txt");
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing.trim().is_empty() || existing.contains("CHANGE_ME") {
        let tok = format!("pb_admin_{}", hex::encode(rand_bytes(24)));
        std::fs::write(path, format!("{}
", tok)).map_err(|e| e.to_string())?;
        self.write_log(id, "Admin token rotated and written to /state/admin_token.txt");
    } else {
        self.write_log(id, "Admin token already set; leaving unchanged.");
    }
    self.end_step(s, id, StepStatus::Success, None);
    Ok(())
}
            _ => {
                self.end_step(s, id, StepStatus::Fail, Some("Unknown step".into()));
                Err("Unknown step".into())
            }
        }
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn get_features(State(st): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = st.state_dir.join("features.json");
    if !p.exists() {
        fs::write(&p, serde_json::to_string_pretty(&serde_json::json!({"flags":{"exchange":false,"proposals":false,"marketplace":false,"oracle":false}})).unwrap()).ok();
    }
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(p).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(v))
}

async fn set_feature(Path((name, state)): Path<(String, String)>, State(st): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = st.state_dir.join("features.json");
    let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&p).unwrap_or_else(|_| "{"flags":{}}".into()))
        .unwrap_or_else(|_| serde_json::json!({"flags":{}}));
    v["flags"][name] = serde_json::Value::Bool(state == "on");
    fs::write(&p, serde_json::to_string_pretty(&v).unwrap()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok":true,"flags":v["flags"]})))
}

async fn status(State(st): State<AppState>) -> Json<RunState> {
    let eng = st.engine.lock().await;
    Json(eng.read_state())
}

async fn output(State(st): State<AppState>) -> Json<serde_json::Value> {
    let deploy_p = st.state_dir.join("deploy.json");
    let deploy: serde_json::Value = if deploy_p.exists() {
        serde_json::from_str(&fs::read_to_string(deploy_p).unwrap_or_else(|_| "{}".into())).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let pk_p = st.state_dir.join("deployer.privatekey");
    let pk = if pk_p.exists() { fs::read_to_string(pk_p).unwrap_or_default().trim().to_string() } else { "".into() };

    Json(serde_json::json!({
        "ok": true,
        "deploy": deploy,
        "deployer_private_key": pk
    }))
}

async fn logs(Path(step): Path<String>, State(st): State<AppState>) -> Json<serde_json::Value> {
    let p = st.state_dir.join("logs").join(format!("{step}.log"));
    let log = if p.exists() { fs::read_to_string(p).unwrap_or_default() } else { "".into() };
    Json(serde_json::json!({ "ok": true, "step": step, "log": log }))
}



async fn fix_and_retry(State(st): State<AppState>) -> Json<serde_json::Value> {
    let eng = st.engine.lock().await;
    let rs = eng.read_state();
    let failed = rs.run_state.last_failed_step.clone();
    let err = rs.run_state.last_error.clone().unwrap_or_default();
    if failed.is_none() {
        return Json(serde_json::json!({"ok": false, "message":"No failed step recorded"}));
    }
    let fixes = rs.run_state.suggested_fixes.clone().unwrap_or_else(|| infer_fixes_from_error(&err));
    for fx in fixes.iter() { eng.apply_fix(fx); }
    // retry
    drop(eng);
    let eng2 = st.engine.lock().await;
    let mut s2 = eng2.read_state().run_state;
    let step = failed.unwrap();
    if let Err(e) = eng2.run_step(&mut s2, &step).await {
        return Json(serde_json::json!({"ok": false, "message": format!("Retry failed: {}", e), "fixes": fixes}));
    }
    return Json(serde_json::json!({"ok": true, "message":"Fix applied and step retried", "fixes": fixes}));
}

async fn run(State(st): State<AppState>, Json(req): Json<RunReq>) -> Json<serde_json::Value> {
    let clean = req.clean_start.unwrap_or(false);
    let eng = st.engine.lock().await;
    match eng.run_all(clean).await {
        Ok(s) => Json(serde_json::json!({ "ok": true, "state": s })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e, "state": eng.read_state() })),
    }
}

async fn retry(Path(step): Path<String>, State(st): State<AppState>) -> Json<serde_json::Value> {
    let mut eng = st.engine.lock().await;
    let mut s = eng.read_state();
    match eng.run_step(&mut s, &step).await {
        Ok(_) => Json(serde_json::json!({ "ok": true, "state": eng.read_state() })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e, "state": eng.read_state() })),
    }
}

async fn fix(Path(fix_id): Path<String>, State(st): State<AppState>) -> Json<serde_json::Value> {
    let eng = st.engine.lock().await;
    eng.apply_fix(&fix_id);
    Json(serde_json::json!({ "ok": true }))
}

#[tokio::main]
async fn main() {
    let state_dir = PathBuf::from(std::env::var("STATE_DIR").unwrap_or_else(|_| "/state".into()));
    fs::create_dir_all(&state_dir).ok();

    let engine = Engine::new(state_dir.clone());
    let app_state = AppState { state_dir, engine: Arc::new(Mutex::new(engine)) };

    let app = Router::new()
        .route("/health", get(health))
        .route("/features", get(get_features))
        .route("/features/:name/:state", post(set_feature))
        .route("/installer/status", get(status))
        .route("/installer/logs/:step", get(logs))
        .route("/installer/run", post(run))
        .route("/installer/config", post(configure))
        .route("/installer/fix_and_retry", post(fix_and_retry))
        .route("/installer/retry/:step", post(retry))
        .route("/installer/fix/:fix_id", post(fix))
        .route("/installer/output", get(output))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8085);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    println!("press_deployer_api listening on {addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

fn read_deploy_json() -> Option<serde_json::Value> {
    let p = "/state/deploy.json";
    let s = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}

fn rand_bytes(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).ok();
    b
}


fn infer_fixes_from_error(err: &str) -> Vec<String> {
    let e = err.to_lowercase();
    let mut fixes = Vec::new();
    if e.contains("cannot connect to the docker daemon") || e.contains("is the docker daemon running") {
        fixes.push("start_docker".into());
    }
    if e.contains("permission denied") && (e.contains("/state") || e.contains("/repo/state")) {
        fixes.push("chmod_state".into());
    }
    if e.contains("network") && e.contains("not found") {
        fixes.push("recreate_network".into());
    }
    if e.contains("cmd failed: cd /repo && docker compose") {
        fixes.push("compose_down".into());
        fixes.push("recreate_network".into());
    }
    if fixes.is_empty() {
        fixes.push("clean_orphans".into());
    }
    fixes
}


fn ensure_listing_tiers(engine: &Engine, step_id: &str) -> Result<(), String> {
    let st = std::path::Path::new("/state/listing_tiers.json");
    if st.exists() { return Ok(()); }
    let cfg = std::path::Path::new("/app/config/listing_tiers.json");
    if cfg.exists() {
        std::fs::copy(cfg, st).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(st, std::fs::Permissions::from_mode(0o644));
        }
        engine.write_log(step_id, "Seeded /state/listing_tiers.json from config.");
    }
    Ok(())
}


fn read_feature_flags() -> serde_json::Value {
    let s = std::fs::read_to_string("/state/feature_flags.json")
        .or_else(|_| std::fs::read_to_string("/app/config/feature_flags.json"))
        .unwrap_or_else(|_| "{}".into());
    serde_json::from_str(&s).unwrap_or(serde_json::json!({"version":"unknown","features":{}}))
}

fn write_feature_flags(v: &serde_json::Value) -> Result<(), String> {
    std::fs::write("/state/feature_flags.json", serde_json::to_string_pretty(v).unwrap()).map_err(|e| e.to_string())
}

async fn installer_features_get() -> Json<serde_json::Value> {
    Json(read_feature_flags())
}

#[derive(Deserialize)]
struct FeatureSetReq {
    key: String,
    enabled: bool
}

async fn installer_features_set(Json(req): Json<FeatureSetReq>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    if req.key.trim().is_empty() { return Err((axum::http::StatusCode::BAD_REQUEST, "key required".into())); }
    let mut v = read_feature_flags();
    let feats = v.get_mut("features").and_then(|x| x.as_object_mut()).ok_or((axum::http::StatusCode::BAD_REQUEST,"features missing".into()))?;
    let entry = feats.entry(req.key.clone()).or_insert(serde_json::json!({"enabled": true}));
    if let Some(obj)=entry.as_object_mut() {
        obj.insert("enabled".into(), serde_json::json!(req.enabled));
    }
    // bump version stamp
    v["version"] = serde_json::json!("RR41-runtime");
    write_feature_flags(&v).map_err(|e|(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({"ok": true, "key": req.key, "enabled": req.enabled})))
}

async fn installer_features_page() -> impl IntoResponse {
    let html = r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>Press Deployer — Feature Toggles</title>
  <style>
    html,body{margin:0;padding:0;background:#05060A;color:#E5E7EB;font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Arial}
    .wrap{max-width:1060px;margin:0 auto;padding:26px}
    .h{display:flex;gap:12px;align-items:center}
    .logo{width:44px;height:44px;border-radius:14px;background:linear-gradient(135deg,#22D3EE,#A78BFA);box-shadow:0 10px 35px rgba(34,211,238,.18)}
    .title{font-weight:950;font-size:22px}
    .sub{color:#94A3B8;font-size:12px;margin-top:2px;line-height:1.6}
    .card{margin-top:16px;padding:16px;border-radius:18px;border:1px solid rgba(148,163,184,.14);background:linear-gradient(180deg,rgba(15,23,42,.62),rgba(2,6,23,.62));box-shadow:0 18px 40px rgba(0,0,0,.25)}
    .row{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;align-items:center;padding:12px 0;border-bottom:1px solid rgba(148,163,184,.10)}
    .row:last-child{border-bottom:none}
    .k{font-weight:900}
    .d{color:#94A3B8;font-size:12px;margin-top:2px}
    .pill{padding:10px 12px;border-radius:999px;border:1px solid rgba(148,163,184,.18);background:rgba(15,23,42,.55);color:#E5E7EB;font-size:12px;cursor:pointer}
    .pill:hover{border-color:rgba(125,211,252,.35)}
    .ok{color:#86EFAC}
    .warn{color:#FBBF24}
    a{color:#7DD3FC;text-decoration:none}
  </style>
</head>
<body>
  <div class="wrap">
    <div class="h">
      <div class="logo"></div>
      <div>
        <div class="title">Feature Toggles</div>
        <div class="sub">Everything is enabled by default. Toggle modules off for staged marketing releases (“coming soon”), while keeping the full stack deployed.</div>
      </div>
    </div>

    <div class="card" id="card">
      Loading…
    </div>

    <div class="sub" style="margin-top:14px;">
      Notes:
      <ul>
        <li>These toggles update <code>/state/feature_flags.json</code>.</li>
        <li>Gateway enforces module routing: disabled modules return 404.</li>
        <li>Premium APIs also have their own flags (advanced_analytics, syndication_marketplace).</li>
      </ul>
      Quick links: <a href="/exchange">/exchange</a> · <a href="/outlet">/outlet</a>
    </div>
  </div>

<script>
async function load(){
  const card=document.getElementById("card");
  const v=await fetch("/installer/features/get").then(r=>r.json());
  const feats=v.features||{};
  const rows=Object.keys(feats).sort().map(k=>{
    const f=feats[k]||{};
    const en=!!f.enabled;
    const desc=f.desc||"";
    const route=f.route||"";
    return {k,en,desc,route};
  });
  card.innerHTML = rows.map(r=>`
    <div class="row">
      <div>
        <div class="k">${r.k} <span class="${r.en?'ok':'warn'}">${r.en?'ENABLED':'DISABLED'}</span></div>
        <div class="d">${r.desc} ${r.route?` · <span style="color:#64748B">${r.route}</span>`:''}</div>
      </div>
      <button class="pill" onclick="toggle('${r.k}', ${r.en?'false':'true'})">${r.en?'Disable':'Enable'}</button>
    </div>
  `).join("");
}
async function toggle(key, enabled){
  await fetch("/installer/features/set",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({key,enabled})});
  await load();
}
load();
</script>
</body></html>"#;
    axum::response::Html(html)
}


fn read_state_string(file: &str) -> String {
    let p = format!("/state/{}", file);
    std::fs::read_to_string(&p).unwrap_or_default().trim().to_string()
}


fn ensure_proposal_presets(engine: &Engine, step_id: &str) -> Result<(), String> {
    let st = std::path::Path::new("/state/proposal_presets.json");
    if st.exists() { return Ok(()); }
    let cfg = std::path::Path::new("/app/config/proposal_presets.json");
    if cfg.exists() {
        std::fs::copy(cfg, st).map_err(|e| e.to_string())?;
        engine.write_log(step_id, "Seeded /state/proposal_presets.json from config.");
    }
    Ok(())
}


fn ensure_treasury_key(engine: &Engine, step_id: &str) -> Result<(), String> {
    let p = std::path::Path::new("/state/press_treasury_private_key.txt");
    if p.exists() { return Ok(()); }
    // re-use deployer key for MVP if dedicated treasury key is not set.
    let d = std::path::Path::new("/state/deployer_private_key.txt");
    if d.exists() {
        std::fs::copy(d, p).map_err(|e| e.to_string())?;
        engine.write_log(step_id, "Seeded /state/press_treasury_private_key.txt from deployer key (MVP). Replace with dedicated treasury key in production.");
    }
    Ok(())
}
