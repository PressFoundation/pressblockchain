<?php
/**
 * Plugin Name: Press SYNC (Outlet Mode)
 * Description: Converts WordPress into a Press Blockchain-native outlet: on-chain article provenance, role-gated voting, syndication, marketplace hooks, tipping, and zero-barrier outlet onboarding.
 * Version: 0.2.0
 * Author: Press Labs Inc.
 */
if (!defined('ABSPATH')) { exit; }

final class PressSyncOutletMode {
  const OPT_GROUP = 'press_sync';
  const OPT_PREFIX = 'press_sync_';
  const VERSION = '0.2.0';

  public function __construct() {
    add_action('admin_menu', [$this, 'menu']);
    add_action('admin_init', [$this, 'settings']);
    add_action('admin_enqueue_scripts', [$this, 'enqueue_admin']);
    add_action('init', [$this, 'register_shortcodes']);

    // Admin UX: Press Mode (collapse legacy WP menus by default)
    add_action('admin_body_class', [$this, 'admin_body_class']);
    add_action('admin_head', [$this, 'admin_head']);
    add_action('wp_dashboard_setup', [$this, 'dashboard_widget']);

    // AJAX (server-to-gateway) actions
    add_action('wp_ajax_press_sync_toggle_mode', [$this, 'ajax_toggle_mode']);
    add_action('wp_ajax_press_sync_refresh_status', [$this, 'ajax_refresh_status']);
    add_action('wp_ajax_press_sync_create_outlet', [$this, 'ajax_create_outlet']);
    add_action('wp_ajax_press_sync_deploy_outlet_token', [$this, 'ajax_deploy_outlet_token']);
    add_action('wp_ajax_press_sync_list_token', [$this, 'ajax_list_token']);

    // Core editorial hooks (MVP: attach on-chain metadata to posts; on-chain register can be enabled when gateway implements it)
    add_action('save_post', [$this, 'on_save_post'], 10, 3);
    add_filter('the_content', [$this, 'inject_press_widgets']);
  }

  /* ============================================================
   *  SYNC MVP: 8 flagship features
   *  1) Press Admin UX Mode: collapse default WP menus, replace with Press-first dashboard.
   *  2) Outlet Onboarding Wizard: create outlet + deploy outlet token + list token (tiered) from WP.
   *  3) On-Chain Article Layer: provenance metadata + Article Vote Bar (live counts).
   *  4) Roles & Rights: outlet manager assigns Press roles (editor/reporter/fact-checker/etc) in a dedicated panel.
   *  5) Monetization Suite: tipping widget, co-author revenue split rules (primary/secondary).
   *  6) Syndication & Marketplace: toggles, licensing presets, and distribution hooks.
   *  7) Arweave Import Hub: queue imports and mark as “Arweave-origin” with flags + fees (gateway integration staged).
   *  8) Security & Compliance: optional Press Pass (KYC-like), audit log stub, and hardened defaults.
   * ============================================================ */

  /* =======================
   * Config / helpers
   * ======================= */
  private function opt($k, $default='') {
    return get_option(self::OPT_PREFIX.$k, $default);
  }
  private function set_opt($k, $v) {
    update_option(self::OPT_PREFIX.$k, $v);
  }
  private function gateway() {
    $gw = $this->opt('gateway', 'https://deploy.pressblockchain.io');
    return rtrim($gw, '/');
  }
  private function ajax_ok($data=[]) {
    wp_send_json(array_merge(['ok'=>true], $data));
  }
  private function ajax_fail($msg, $extra=[]) {
    wp_send_json(array_merge(['ok'=>false, 'error'=>$msg], $extra));
  }
  private function require_nonce() {
    $nonce = isset($_POST['_ajax_nonce']) ? sanitize_text_field($_POST['_ajax_nonce']) : '';
    if (!wp_verify_nonce($nonce, 'press_sync_nonce')) {
      $this->ajax_fail('Invalid nonce');
    }
  }
  private function gw_post($path, $body) {
    $url = $this->gateway().$path;
    $res = wp_remote_post($url, [
      'headers' => ['Content-Type'=>'application/json'],
      'timeout' => 30,
      'body' => wp_json_encode($body),
    ]);
    if (is_wp_error($res)) { return ['ok'=>false, 'error'=>$res->get_error_message()]; }
    $txt = wp_remote_retrieve_body($res);
    $json = json_decode($txt, true);
    if (!$json) { $json = ['ok'=>false, 'error'=>"Non-JSON response: ".$txt]; }
    return $json;
  }
  private function gw_get($path) {
    $url = $this->gateway().$path;
    $res = wp_remote_get($url, ['timeout'=>20]);
    if (is_wp_error($res)) { return ['ok'=>false, 'error'=>$res->get_error_message()]; }
    $txt = wp_remote_retrieve_body($res);
    $json = json_decode($txt, true);
    if (!$json) { $json = ['ok'=>false, 'error'=>"Non-JSON response: ".$txt]; }
    return $json;
  }

  /* =======================
   * Admin UX: “Press Mode”
   * ======================= */
  public function admin_body_class($classes) {
    if (current_user_can('manage_options') && $this->opt('press_mode_enabled', '1') === '1') {
      $classes .= ' press-sync-mode';
    }
    return $classes;
  }

  public function admin_head() {
    if (!current_user_can('manage_options')) { return; }
    if ($this->opt('press_mode_enabled', '1') !== '1') { return; }
    remove_menu_page('edit-comments.php');
    remove_menu_page('tools.php');
    remove_menu_page('plugins.php');
    remove_menu_page('users.php');
    remove_menu_page('themes.php');
    remove_menu_page('options-general.php');
  }

  public function enqueue_admin($hook) {
    if (!current_user_can('manage_options')) { return; }
    if (strpos($hook, 'press-sync') === false) { return; }
    wp_enqueue_style('press-sync-admin', plugins_url('assets/admin.css', __FILE__), [], self::VERSION);
    wp_enqueue_script('press-sync-admin', plugins_url('assets/admin.js', __FILE__), [], self::VERSION, true);
    wp_localize_script('press-sync-admin', 'PressSync', [
      'ajax_url' => admin_url('admin-ajax.php'),
      'nonce' => wp_create_nonce('press_sync_nonce'),
    ]);
  }

  public function dashboard_widget() {
    wp_add_dashboard_widget('press_sync_widget', 'Press SYNC — Outlet Status', [$this,'render_dashboard_widget']);
  }
  public function render_dashboard_widget() {
    $st = $this->compute_status();
    echo '<div class="press-sync-mono">'.esc_html(wp_json_encode($st, JSON_PRETTY_PRINT)).'</div>';
    echo '<p class="description">Press Mode makes this your primary admin view. All Press features are enabled by default, with staging flags controlled in the core deployer.</p>';
  }

  /* =======================
   * Admin Menus (Press-first)
   * ======================= */
  public function menu() {
    add_menu_page('Press SYNC', 'Press SYNC', 'manage_options', 'press-sync', [$this,'page_dashboard'], 'dashicons-megaphone', 3);
    add_submenu_page('press-sync', 'Dashboard', 'Dashboard', 'manage_options', 'press-sync', [$this,'page_dashboard']);
    add_submenu_page('press-sync', 'Outlet Wizard', 'Outlet Wizard', 'manage_options', 'press-sync-outlet', [$this,'page_outlet_wizard']);
    add_submenu_page('press-sync', 'On-Chain Publishing', 'On-Chain Publishing', 'manage_options', 'press-sync-publishing', [$this,'page_publishing']);
    add_submenu_page('press-sync', 'Roles & Rights', 'Roles & Rights', 'manage_options', 'press-sync-roles', [$this,'page_roles']);
    add_submenu_page('press-sync', 'Monetization', 'Monetization', 'manage_options', 'press-sync-monetization', [$this,'page_monetization']);
    add_submenu_page('press-sync', 'Syndication & Marketplace', 'Syndication & Marketplace', 'manage_options', 'press-sync-syndication', [$this,'page_syndication']);
    add_submenu_page('press-sync', 'Arweave Import', 'Arweave Import', 'manage_options', 'press-sync-arweave', [$this,'page_arweave']);
    add_submenu_page('press-sync', 'Settings', 'Settings', 'manage_options', 'press-sync-settings', [$this,'page_settings']);
  }

  /* =======================
   * Settings (single source)
   * ======================= */
  public function settings() {
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'gateway');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'outlet_domain');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'outlet_wallet');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'license_tier');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'treasury_vault');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'tip_router');

    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'press_mode_enabled');

    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'press_pass_enabled');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'press_pass_min_level');

    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'syndication_enabled');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'default_license_price');

    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'arweave_import_fee');
    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'arweave_import_bond');

    register_setting(self::OPT_GROUP, self::OPT_PREFIX.'coauthor_fee_press_wei');
  }

  /* =======================
   * STATUS / KPIs
   * ======================= */
  private function compute_status() {
    $info = $this->gw_get('/api/outlet/info');
    return [
      'gateway' => $this->gateway(),
      'outlet_domain' => $this->opt('outlet_domain',''),
      'outlet_wallet' => $this->opt('outlet_wallet',''),
      'license_tier' => $this->opt('license_tier','standard'),
      'press_mode_enabled' => $this->opt('press_mode_enabled','1') === '1',
      'press_pass_enabled' => $this->opt('press_pass_enabled','0') === '1',
      'syndication_enabled' => $this->opt('syndication_enabled','1') === '1',
      'contracts' => $info,
      'note' => 'If contracts are empty, deployer stack may not be running or gateway is misconfigured.',
    ];
  }

  /* =======================
   * AJAX
   * ======================= */
  public function ajax_toggle_mode() {
    $this->require_nonce();
    if (!current_user_can('manage_options')) { $this->ajax_fail('Forbidden'); }
    $enabled = $this->opt('press_mode_enabled','1') === '1' ? '0' : '1';
    $this->set_opt('press_mode_enabled', $enabled);
    $this->ajax_ok(['enabled' => $enabled === '1']);
  }

  public function ajax_refresh_status() {
    $this->require_nonce();
    if (!current_user_can('manage_options')) { $this->ajax_fail('Forbidden'); }
    $this->ajax_ok($this->compute_status());
  }

  public function ajax_create_outlet() {
    $this->require_nonce();
    if (!current_user_can('manage_options')) { $this->ajax_fail('Forbidden'); }
    $payload = json_decode(stripslashes($_POST['payload'] ?? '{}'), true);
    $name = sanitize_text_field($payload['name'] ?? '');
    $domain = sanitize_text_field($payload['domain'] ?? '');
    $owner_pk = isset($payload['owner_private_key']) ? sanitize_text_field($payload['owner_private_key']) : null;
    if (!$name || !$domain) { $this->ajax_fail('Missing name/domain'); }
    $r = $this->gw_post('/api/outlets/create', ['name'=>$name,'domain'=>$domain,'owner_private_key'=>$owner_pk]);
    if (!empty($r['ok'])) { $this->set_opt('outlet_domain', $domain); }
    wp_send_json($r);
  }

  public function ajax_deploy_outlet_token() {
    $this->require_nonce();
    if (!current_user_can('manage_options')) { $this->ajax_fail('Forbidden'); }
    $payload = json_decode(stripslashes($_POST['payload'] ?? '{}'), true);
    $domain = sanitize_text_field($payload['domain'] ?? '');
    if (!$domain) { $domain = $this->opt('outlet_domain',''); }
    $r = $this->gw_post('/api/outlets/token/deploy', [
      'domain' => $domain,
      'token_name' => sanitize_text_field($payload['token_name'] ?? ''),
      'token_symbol' => sanitize_text_field($payload['token_symbol'] ?? ''),
      'minted_supply_wei' => sanitize_text_field($payload['minted_supply_wei'] ?? '0'),
      'test_transfer_to_self_wei' => sanitize_text_field($payload['test_transfer_to_self_wei'] ?? '0'),
      'owner_private_key' => isset($payload['owner_private_key']) ? sanitize_text_field($payload['owner_private_key']) : null,
    ]);
    wp_send_json($r);
  }

  public function ajax_list_token() {
    $this->require_nonce();
    if (!current_user_can('manage_options')) { $this->ajax_fail('Forbidden'); }
    $payload = json_decode(stripslashes($_POST['payload'] ?? '{}'), true);
    $domain = sanitize_text_field($payload['domain'] ?? '');
    if (!$domain) { $domain = $this->opt('outlet_domain',''); }
    $r = $this->gw_post('/api/exchange/list', [
      'domain' => $domain,
      'token_address' => sanitize_text_field($payload['token_address'] ?? ''),
      'tier' => intval($payload['tier'] ?? 1),
      'owner_private_key' => isset($payload['owner_private_key']) ? sanitize_text_field($payload['owner_private_key']) : null,
    ]);
    wp_send_json($r);
  }

  /* =======================
   * Pages
   * ======================= */
  private function shell_open($title, $subtitle) {
    echo '<div class="wrap"><div class="press-sync-shell">';
    echo '<div class="press-sync-top">';
    echo '<div style="display:flex;gap:12px;align-items:center">';
    echo '<div class="press-sync-logo"></div>';
    echo '<div><div class="press-sync-h1">'.esc_html($title).'</div>';
    echo '<div class="press-sync-sub">'.esc_html($subtitle).'</div></div>';
    echo '</div>';
    echo '<div class="press-sync-row">';
    echo '<button type="button" class="press-sync-btn2" id="pressSyncToggleMode">Toggle Press Mode</button>';
    echo '<button type="button" class="press-sync-btn2" id="pressSyncQuickRefresh">Refresh Status</button>';
    echo '</div></div>';
    echo '<div class="press-sync-band"></div>';
  }
  private function shell_close() { echo '</div></div>'; }

  public function page_dashboard() {
    $this->shell_open('Press SYNC Dashboard', 'This plugin is the WordPress MVP: it makes your outlet feel native to Press Blockchain. Default WordPress admin is collapsed; Press tools become the primary operating system for the outlet.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Outlet Status <span class="press-sync-tag good">Live</span></div>';
    echo '<div class="press-sync-hint">Everything is enabled by default. Use the core deployer to stage features (exchange/proposals/court) for future marketing.</div>';
    echo '<div class="press-sync-row"><div class="press-sync-mono" id="pressSyncStatus">'.esc_html(wp_json_encode($this->compute_status(), JSON_PRETTY_PRINT)).'</div></div>';
    echo '</div>';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Flagship Features <span class="press-sync-tag">8</span></div>';
    echo '<ul style="margin:10px 0 0 18px;line-height:1.55;opacity:.92">';
    echo '<li><strong>Press Admin UX Mode</strong> (collapses WP defaults; Press-first navigation)</li>';
    echo '<li><strong>Outlet Wizard</strong> (create outlet + deploy outlet token + list tiers)</li>';
    echo '<li><strong>On-Chain Articles</strong> (provenance metadata + live vote bar)</li>';
    echo '<li><strong>Roles & Rights</strong> (Press roles mapped to WP users)</li>';
    echo '<li><strong>Monetization</strong> (tipping + co-author split presets)</li>';
    echo '<li><strong>Syndication & Marketplace</strong> (licensing presets + distribution hooks)</li>';
    echo '<li><strong>Arweave Import Hub</strong> (queue imports + flags/fees)</li>';
    echo '<li><strong>Security & Compliance</strong> (optional Press Pass, audit log stub)</li>';
    echo '</ul>';
    echo '</div>';
    echo '</div>';
    $this->shell_close();
  }

  public function page_outlet_wizard() {
    $this->shell_open('Outlet Wizard', 'Create your official outlet identity, deploy your outlet token with a required test transaction, and list on the Press Exchange with tiered perks — directly from WordPress.');
    $domain = esc_attr($this->opt('outlet_domain',''));
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Step 1 — Outlet Identity <span class="press-sync-tag">Required</span></div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Outlet Name</label><input id="ps_outlet_name" placeholder="Example: Press News San Diego" /></div>';
    echo '<div><label>Official Domain (members see this)</label><input id="ps_outlet_domain" value="'.$domain.'" placeholder="pressnewssd.com" /></div>';
    echo '</div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Owner Private Key (bootstrap mode)</label><input id="ps_owner_pk" type="password" placeholder="0x... (optional if server has owner_keys.json)" /></div>';
    echo '<div><label>Outlet ID (auto)</label><input id="ps_outlet_id" class="press-sync-mono" disabled placeholder="auto" /></div>';
    echo '</div>';
    echo '<div class="press-sync-row"><button type="button" class="press-sync-btn" id="pressSyncCreateOutlet">Create Outlet (on-chain)</button></div>';
    echo '<pre class="press-sync-mono" id="pressSyncOut" style="margin-top:10px">—</pre>';
    echo '</div>';

    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Step 2 — Outlet Token + Required Test Tx <span class="press-sync-tag good">Recommended</span></div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Token Name</label><input id="ps_tok_name" placeholder="Example: PRESS NEWS SD" /></div>';
    echo '<div><label>Token Symbol</label><input id="ps_tok_symbol" placeholder="PNUSD" /></div>';
    echo '</div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Minted Supply (wei)</label><input id="ps_tok_supply" value="1000000000000000000000000" /></div>';
    echo '<div><label>Test Tx Amount (wei)</label><input id="ps_tok_test" value="1000000000000000000" /></div>';
    echo '</div>';
    echo '<div class="press-sync-row"><button type="button" class="press-sync-btn" id="pressSyncDeployToken">Deploy Token + Run Test</button></div>';
    echo '<pre class="press-sync-mono" id="pressSyncTokOut" style="margin-top:10px">—</pre>';
    echo '</div>';

    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Step 3 — Exchange Listing Tiers <span class="press-sync-tag warn">Optional</span></div>';
    echo '<div class="press-sync-hint">Three listing tiers provide escalating perks (credibility badges, syndication priority, analytics, liquidity tooling). Listing is a revenue driver and a utility upgrade for outlets.</div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Token Address</label><input id="ps_list_token" placeholder="0x..." /></div>';
    echo '<div><label>Tier</label><select id="ps_list_tier"><option value="1">Basic</option><option value="2">Pro</option><option value="3">Elite</option></select></div>';
    echo '</div>';
    echo '<div class="press-sync-row"><button type="button" class="press-sync-btn" id="pressSyncListToken">List Token (on-chain)</button></div>';
    echo '<pre class="press-sync-mono" id="pressSyncListOut" style="margin-top:10px">—</pre>';
    echo '</div>';

    echo '</div>';
    $this->shell_close();
  }

  public function page_publishing() {
    $this->shell_open('On-Chain Publishing', 'Posts become on-chain provenance records. Articles can be voted/approved in a 72h window, and the vote bar can be embedded anywhere.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Article Provenance <span class="press-sync-tag">MVP</span></div>';
    echo '<div class="press-sync-hint">On save, we attach Press metadata to posts. When the gateway exposes an “article register” endpoint, this plugin will push the on-chain create automatically (feature flag-ready).</div>';
    echo '<div class="press-sync-row"><span class="press-sync-tag">Shortcodes</span></div>';
    echo '<div class="press-sync-mono">[press_votebar article="ARTICLE_ID_HEX"]  — live counts (polling gateway)</div>';
    echo '<div class="press-sync-mono">[press_tip article="ARTICLE_ID_HEX"]      — tipping widget (PRESS/ETH/USDC/WBTC)</div>';
    echo '<div class="press-sync-mono">[press_proof article="ARTICLE_ID_HEX"]    — provenance badge (hash + outlet domain)</div>';
    echo '</div>';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Default Vote Window <span class="press-sync-tag good">72h</span></div>';
    $a = $this->gw_get('/api/articles/approval_defaults');
    echo '<div class="press-sync-mono">'.esc_html(wp_json_encode($a, JSON_PRETTY_PRINT)).'</div>';
    echo '<div class="press-sync-hint">These numbers are configured in the core deployer. This panel surfaces them so editors can plan publishing workflows.</div>';
    echo '</div>';
    echo '</div>';
    $this->shell_close();
  }

  public function page_roles() {
    $this->shell_open('Roles & Rights', 'Outlet managers can assign Press roles to WordPress users. Default WP roles are treated as legacy and secondary.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Press Roles (preset)</div>';
    echo '<div class="press-sync-hint">This is the MVP role map. Next pass can bind these to on-chain role NFTs/bonds so “role = on-chain” not just a WP setting.</div>';
    echo '<ul style="margin:10px 0 0 18px;line-height:1.6;opacity:.92">';
    echo '<li>Outlet Manager (full controls)</li><li>Editor (publishing + approvals)</li><li>Reporter (drafts + submissions)</li><li>Fact Checker (evidence + flags)</li><li>Researcher (source linking + citations)</li><li>Legal (court submissions for outlet)</li><li>Analyst (data + syndication tracking)</li>';
    echo '</ul>';
    echo '</div>';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Assign Role (local MVP)</div>';
    echo '<div class="press-sync-hint">Select a WP user and assign a Press role. Stored as user meta: press_sync_role. This makes the interface Press-first immediately.</div>';
    $users = get_users(['number'=>50,'orderby'=>'registered','order'=>'DESC']);
    echo '<form method="post">';
    wp_nonce_field('press_sync_assign_role', 'press_sync_assign_role_nonce');
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>User</label><select name="ps_user">';
    foreach($users as $u){ echo '<option value="'.esc_attr($u->ID).'">'.esc_html($u->user_login.' ('.$u->user_email.')').'</option>'; }
    echo '</select></div>';
    echo '<div><label>Press Role</label><select name="ps_role">';
    foreach(['outlet_manager','editor','reporter','fact_checker','researcher','legal','analyst'] as $r){ echo '<option value="'.esc_attr($r).'">'.esc_html(str_replace('_',' ', $r)).'</option>'; }
    echo '</select></div></div>';
    echo '<div class="press-sync-row"><button class="press-sync-btn" type="submit" name="ps_assign_role" value="1">Assign</button></div>';
    echo '</form>';
    if (isset($_POST['ps_assign_role']) && check_admin_referer('press_sync_assign_role','press_sync_assign_role_nonce')) {
      $uid=intval($_POST['ps_user'] ?? 0);
      $role=sanitize_text_field($_POST['ps_role'] ?? '');
      if ($uid && $role) { update_user_meta($uid, 'press_sync_role', $role); echo '<div class="press-sync-tag good" style="margin-top:10px;display:inline-block">Assigned</div>'; }
    }
    echo '</div></div>';
    $this->shell_close();
  }

  public function page_monetization() {
    $this->shell_open('Monetization', 'Built-in tipping (PRESS/ETH/USDC/WBTC), co-author splits, and licensing presets. Authors keep 100% of article revenue; protocol/treasury fees are enforced at the chain level.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Tipping Widget <span class="press-sync-tag good">Enabled</span></div>';
    echo '<div class="press-sync-hint">Embed [press_tip article="..."] on any post/page. Requires Press Wallet login (staged) and supports tiny tips. Non-role wallets can be required to hold a bond (configured at chain layer).</div>';
    echo '<div class="press-sync-mono">Shortcode: [press_tip article="0xARTICLEID"]</div>';
    echo '</div>';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Co‑Author Split (50/50) <span class="press-sync-tag">Preset</span></div>';
    echo '<div class="press-sync-hint">Co-author is added for a small PRESS fee (configured here; enforced on-chain in later pass). Primary keeps control rights; revenue splits permanently 50/50, including rights sales.</div>';
    echo '<form method="post" action="options.php">';
    settings_fields(self::OPT_GROUP);
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Co‑Author Add Fee (PRESS wei)</label><input name="'.esc_attr(self::OPT_PREFIX.'coauthor_fee_press_wei').'" value="'.esc_attr($this->opt('coauthor_fee_press_wei','1000000000000000000')).'"/></div>';
    echo '<div><label>Default License Price (PRESS wei)</label><input name="'.esc_attr(self::OPT_PREFIX.'default_license_price').'" value="'.esc_attr($this->opt('default_license_price','5000000000000000000')).'"/></div>';
    echo '</div>';
    submit_button('Save Monetization Presets');
    echo '</form>';
    echo '</div></div>';
    $this->shell_close();
  }

  public function page_syndication() {
    $this->shell_open('Syndication & Marketplace', 'Turn your outlet into a distribution engine: syndication feeds, licensing presets, and marketplace discoverability with anti-spam safeguards.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Syndication Controls</div>';
    echo '<form method="post" action="options.php">';
    settings_fields(self::OPT_GROUP);
    $enabled = $this->opt('syndication_enabled','1');
    echo '<div class="press-sync-row"><label style="display:flex;gap:10px;align-items:center"><input type="checkbox" name="'.esc_attr(self::OPT_PREFIX.'syndication_enabled').'" value="1" '.checked($enabled,'1',false).'/> Enable Syndication</label></div>';
    echo '<div class="press-sync-hint">When enabled, this outlet can publish syndication metadata and opt into marketplace distribution tiers.</div>';
    submit_button('Save Syndication Settings');
    echo '</form>';
    echo '</div>';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Marketplace Presets <span class="press-sync-tag warn">Hooks</span></div>';
    echo '<div class="press-sync-hint">This MVP stores presets locally and surfaces them to the outlet. Next pass binds marketplace listings to on-chain registry + token tier perks.</div>';
    echo '<ul style="margin:10px 0 0 18px;line-height:1.6;opacity:.92">';
    echo '<li>Licensing preset: non-exclusive, 30 days, auto-renew optional</li>';
    echo '<li>Credibility badge: Press Oracle confidence score (when Oracle enabled)</li>';
    echo '<li>Distribution rail: syndication bundle for partner outlets</li>';
    echo '</ul>';
    echo '</div></div>';
    $this->shell_close();
  }

  public function page_arweave() {
    $this->shell_open('Arweave Import', 'Import legacy archives from Arweave and convert them into Press-native on-chain article records with “Arweave-origin” flags, metadata, and provenance.');
    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Import Queue <span class="press-sync-tag">MVP</span></div>';
    echo '<div class="press-sync-hint">Paste Arweave TX IDs (one per line). Imports require a small PRESS fee + bond so the importer has PRESS to cover publishing and long-term availability signals.</div>';
    echo '<form method="post">';
    wp_nonce_field('press_sync_arweave_import', 'press_sync_arweave_nonce');
    echo '<label>Arweave TX IDs</label><textarea name="ps_arweave_txs" placeholder="TXID1\nTXID2\n..."></textarea>';
    echo '<div class="press-sync-row"><button class="press-sync-btn" type="submit" name="ps_arweave_submit" value="1">Queue Imports</button></div>';
    echo '</form>';

    if (isset($_POST['ps_arweave_submit']) && check_admin_referer('press_sync_arweave_import','press_sync_arweave_nonce')) {
      $raw = sanitize_textarea_field($_POST['ps_arweave_txs'] ?? '');
      $lines = array_filter(array_map('trim', preg_split('/\R/', $raw)));
      $q = get_option('press_sync_arweave_queue', []);
      if (!is_array($q)) { $q=[]; }
      foreach($lines as $tx){ $q[]=['tx'=>$tx,'ts'=>time(),'status'=>'queued']; }
      update_option('press_sync_arweave_queue', $q);
      echo '<div class="press-sync-tag good" style="margin-top:10px;display:inline-block">Queued '.count($lines).' items.</div>';
    }

    $q = get_option('press_sync_arweave_queue', []);
    echo '<div class="press-sync-band"></div><div class="press-sync-title">Current Queue <span class="press-sync-tag">'.(is_array($q)?count($q):0).'</span></div>';
    echo '<pre class="press-sync-mono">'.esc_html(wp_json_encode($q, JSON_PRETTY_PRINT)).'</pre>';
    echo '</div>';

    echo '<div class="press-sync-card">';
    echo '<div class="press-sync-title">Why import from Arweave?</div>';
    echo '<ul style="margin:10px 0 0 18px;line-height:1.6;opacity:.92">';
    echo '<li><strong>Credibility upgrade:</strong> legacy archives gain Press Oracle scoring + court/evidence rails.</li>';
    echo '<li><strong>Monetization unlock:</strong> imported articles become licensable and tip-enabled through Press Wallet.</li>';
    echo '<li><strong>Anti-clone defense:</strong> Press binds outlet identity + domain + votes to the content’s provenance.</li>';
    echo '<li><strong>Migration path:</strong> outlets can bring their history without rewriting their CMS stack.</li>';
    echo '</ul>';
    echo '</div></div>';
    $this->shell_close();
  }

  public function page_settings() {
    $this->shell_open('Settings', 'Core configuration. You should not need manual edits elsewhere. These values coordinate the outlet, gateway, and treasury routing.');
    echo '<form method="post" action="options.php">';
    settings_fields(self::OPT_GROUP);

    echo '<div class="press-sync-grid">';
    echo '<div class="press-sync-card"><div class="press-sync-title">Core Connectivity</div>';
    echo '<label>Gateway Base URL</label><input name="'.esc_attr(self::OPT_PREFIX.'gateway').'" value="'.esc_attr($this->opt('gateway','https://deploy.pressblockchain.io')).'"/>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Outlet Domain</label><input name="'.esc_attr(self::OPT_PREFIX.'outlet_domain').'" value="'.esc_attr($this->opt('outlet_domain','')).'"/></div>';
    echo '<div><label>Outlet Owner Wallet</label><input name="'.esc_attr(self::OPT_PREFIX.'outlet_wallet').'" value="'.esc_attr($this->opt('outlet_wallet','')).'"/></div>';
    echo '</div></div>';

    echo '<div class="press-sync-card"><div class="press-sync-title">Press Mode & Compliance</div>';
    $pm = $this->opt('press_mode_enabled','1');
    $pp = $this->opt('press_pass_enabled','0');
    echo '<div class="press-sync-row"><label style="display:flex;gap:10px;align-items:center"><input type="checkbox" name="'.esc_attr(self::OPT_PREFIX.'press_mode_enabled').'" value="1" '.checked($pm,'1',false).'/> Enable Press Mode (collapse default WP admin)</label></div>';
    echo '<div class="press-sync-row"><label style="display:flex;gap:10px;align-items:center"><input type="checkbox" name="'.esc_attr(self::OPT_PREFIX.'press_pass_enabled').'" value="1" '.checked($pp,'1',false).'/> Enable Press Pass (optional)</label></div>';
    echo '<label>Press Pass minimum level (0-3)</label><input name="'.esc_attr(self::OPT_PREFIX.'press_pass_min_level').'" value="'.esc_attr($this->opt('press_pass_min_level','1')).'"/>';
    echo '</div>';

    echo '<div class="press-sync-card"><div class="press-sync-title">Licensing & Treasury Routing</div>';
    $tier = $this->opt('license_tier','standard');
    echo '<label>License Tier</label><select name="'.esc_attr(self::OPT_PREFIX.'license_tier').'">';
    echo '<option value="standard" '.selected($tier,'standard',false).'>Standard</option>';
    echo '<option value="pro" '.selected($tier,'pro',false).'>Pro</option>';
    echo '<option value="enterprise" '.selected($tier,'enterprise',false).'>Enterprise</option>';
    echo '</select>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Treasury Vault</label><input name="'.esc_attr(self::OPT_PREFIX.'treasury_vault').'" value="'.esc_attr($this->opt('treasury_vault','')).'"/></div>';
    echo '<div><label>Tip Router</label><input name="'.esc_attr(self::OPT_PREFIX.'tip_router').'" value="'.esc_attr($this->opt('tip_router','')).'"/></div>';
    echo '</div>';
    echo '<div class="press-sync-hint">Treasury routing is enforced by chain fees/registries; authors keep 100% of article revenue (tips/licensing) by design.</div>';
    echo '</div>';

    echo '<div class="press-sync-card"><div class="press-sync-title">Arweave Import Defaults</div>';
    echo '<div class="press-sync-col2" style="margin-top:10px">';
    echo '<div><label>Import Fee (PRESS wei)</label><input name="'.esc_attr(self::OPT_PREFIX.'arweave_import_fee').'" value="'.esc_attr($this->opt('arweave_import_fee','1000000000000000000')).'"/></div>';
    echo '<div><label>Import Bond (PRESS wei)</label><input name="'.esc_attr(self::OPT_PREFIX.'arweave_import_bond').'" value="'.esc_attr($this->opt('arweave_import_bond','5000000000000000000')).'"/></div>';
    echo '</div></div>';

    echo '</div>';
    submit_button();
    echo '</form>';
    $this->shell_close();
  }

  /* =======================
   * Editorial: attach metadata
   * ======================= */
  public function on_save_post($post_id, $post, $update) {
    if (wp_is_post_autosave($post_id) || wp_is_post_revision($post_id)) { return; }
    if ($post->post_type !== 'post') { return; }

    $domain = $this->opt('outlet_domain','');
    if (!$domain) { return; }

    $content = $post->post_title . "\n" . $post->post_content . "\n" . $post->post_date_gmt;
    $hash = hash('sha256', $content);
    update_post_meta($post_id, 'press_sync_article_hash', $hash);
    update_post_meta($post_id, 'press_sync_outlet_domain', $domain);
  }

  public function inject_press_widgets($content) {
    if (!is_singular('post')) { return $content; }
    $hash = get_post_meta(get_the_ID(), 'press_sync_article_hash', true);
    $domain = get_post_meta(get_the_ID(), 'press_sync_outlet_domain', true);
    if (!$hash || !$domain) { return $content; }

    $badge = '<div style="margin-top:18px;padding:12px 14px;border-radius:14px;border:1px solid rgba(148,163,184,.16);background:rgba(2,6,23,.35);color:#E6F0FF;">'
           . '<strong>Proof of Press</strong> — Outlet: '.esc_html($domain)
           . '<div style="font-size:12px;opacity:.78;margin-top:6px">Article Hash: <span style="font-family:ui-monospace">'.esc_html($hash).'</span></div>'
           . '</div>';
    return $content . $badge;
  }

  /* =======================
   * Shortcodes
   * ======================= */
  public function register_shortcodes() {
    add_shortcode('press_votebar', function($atts){
      $a = shortcode_atts(['article'=>'','gw'=>$this->gateway()], $atts);
      $aid = esc_attr($a['article']);
      $gw = esc_attr(rtrim($a['gw'],'/'));
      if(!$aid) return '<div><em>Missing article id</em></div>';
      ob_start();
      ?>
      <div class="press-votebar" data-article="<?php echo $aid; ?>" data-gw="<?php echo $gw; ?>" style="border:1px solid rgba(148,163,184,.18);border-radius:14px;padding:12px;background:rgba(2,6,23,.35);color:#E6F0FF;margin-top:12px">
        <div style="display:flex;justify-content:space-between;align-items:center;gap:10px;flex-wrap:wrap">
          <div><strong>Press Approval Vote</strong> <span style="font-size:12px;opacity:.75">(72h window)</span></div>
          <div class="press-votebar-status" style="font-size:12px;opacity:.85">Loading…</div>
        </div>
        <div style="display:flex;gap:10px;flex-wrap:wrap;margin-top:10px;font-size:13px">
          <div>Community: <strong class="press-votebar-community">0</strong></div>
          <div>Outlet: <strong class="press-votebar-outlet">0</strong></div>
          <div>Council: <strong class="press-votebar-council">0</strong></div>
          <div>Flags: <strong class="press-votebar-flags">0</strong></div>
        </div>
        <div style="margin-top:8px;font-size:12px;opacity:.75">Counts update live. Voting auto-ends after 72 hours. Voting costs fees on-chain (no free votes).</div>
      </div>
      <script>
      (function(){
        const root=document.currentScript.previousElementSibling;
        if(!root) return;
        const aid=root.getAttribute('data-article');
        const gw=root.getAttribute('data-gw');
        async function tick(){
          try{
            const r=await fetch(gw.replace(/\/$/,'') + '/api/articles/votes/' + aid);
            const j=await r.json();
            if(!j.ok){ root.querySelector('.press-votebar-status').textContent='Not available'; return; }
            root.querySelector('.press-votebar-community').textContent=j.community;
            root.querySelector('.press-votebar-outlet').textContent=j.outlet;
            root.querySelector('.press-votebar-council').textContent=j.council;
            root.querySelector('.press-votebar-flags').textContent=j.flags;
            const now=Math.floor(Date.now()/1000);
            const left=Math.max(0, (j.end_at||0)-now);
            const hrs=Math.floor(left/3600), mins=Math.floor((left%3600)/60);
            const status = j.finalized ? (j.approved ? 'Approved' : 'Not approved') : ('Ends in ' + hrs + 'h ' + mins + 'm');
            root.querySelector('.press-votebar-status').textContent=status;
          }catch(e){
            root.querySelector('.press-votebar-status').textContent='Error';
          }
        }
        tick(); setInterval(tick, 5000);
      })();
      </script>
      <?php
      return ob_get_clean();
    });

    add_shortcode('press_tip', function($atts){
      $a = shortcode_atts(['article'=>''], $atts);
      $aid = esc_attr($a['article']);
      if(!$aid) return '<div><em>Missing article id</em></div>';
      $out = '<div style="border:1px solid rgba(148,163,184,.18);border-radius:14px;padding:12px;background:rgba(2,6,23,.35);color:#E6F0FF;margin-top:12px">';
      $out .= '<div style="display:flex;justify-content:space-between;gap:10px;flex-wrap:wrap;align-items:center">';
      $out .= '<div><strong>Tip this article</strong> <span style="font-size:12px;opacity:.78">(PRESS / ETH / USDC / WBTC)</span></div>';
      $out .= '<div style="font-size:12px;opacity:.78">Requires Press Wallet login</div></div>';
      $out .= '<div style="margin-top:10px;display:flex;gap:10px;flex-wrap:wrap">';
      foreach(['PRESS','ETH','USDC','WBTC'] as $sym){
        $out .= '<button type="button" style="cursor:pointer;padding:8px 10px;border-radius:12px;border:1px solid rgba(148,163,184,.18);background:rgba(15,23,42,.62);color:#E6F0FF;font-weight:900">'.$sym.'</button>';
      }
      $out .= '</div>';
      $out .= '<div style="margin-top:10px;font-size:12px;opacity:.78">Protocol fees route to treasury at the chain layer; authors receive 100% of article revenue. Non-role wallets may require a bond (configured at chain layer).</div>';
      $out .= '</div>';
      return $out;
    });

    add_shortcode('press_proof', function($atts){
      $a = shortcode_atts(['article'=>''], $atts);
      $aid = esc_attr($a['article']);
      $domain = esc_html($this->opt('outlet_domain',''));
      $out = '<div style="border:1px solid rgba(19,196,163,.28);border-radius:14px;padding:12px;background:rgba(19,196,163,.10);color:#E6F0FF;margin-top:12px">';
      $out .= '<strong>Proof of Press</strong> — Verified outlet domain: '.$domain.'<div style="margin-top:6px;font-size:12px;opacity:.78">Article ID: <span style="font-family:ui-monospace">'.($aid?$aid:'(missing)').'</span></div>';
      $out .= '</div>';
      return $out;
    });
  }
}

new PressSyncOutletMode();

require_once __DIR__ . '/includes/rest-votes.php';
