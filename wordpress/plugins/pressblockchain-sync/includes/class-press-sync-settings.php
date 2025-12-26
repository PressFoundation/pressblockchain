<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Settings {
  public static function init() {
    add_action('admin_init', [__CLASS__, 'register_settings']);
  }

  public static function register_settings() {
    register_setting('press_sync', 'press_sync_rpc_url');
    register_setting('press_sync', 'press_sync_outlet_id');
    register_setting('press_sync', 'press_sync_outlet_domain');
    register_setting('press_sync', 'press_sync_press_token');
    register_setting('press_sync', 'press_sync_treasury_wallet');
    register_setting('press_sync', 'press_sync_outlet_mode_enabled');
    register_setting('press_sync', 'press_sync_publish_fee_press');
    register_setting('press_sync', 'press_sync_source_registry');
    register_setting('press_sync', 'press_sync_vote_fee_press');
    register_setting('press_sync', 'press_sync_tip_fee_percent');
    register_setting('press_sync', 'press_sync_ai_endpoint');
    register_setting('press_sync', 'press_sync_installer_api');
    register_setting('press_sync', 'press_sync_treasury_wallet');
    register_setting('press_sync', 'press_sync_outlet_mode_enabled');
  }

  public static function render() {
    ?>
    <div class="wrap press-sync">
      <h1>Press Blockchain SYNC</h1>
      <p class="press-sync-sub">Turn WordPress into a fully connected Press Blockchain outlet. Publishing is invisible: your team uses WordPress as normal while provenance, licensing, tips, and verification are committed on-chain.</p>
      <form method="post" action="options.php">
        <?php settings_fields('press_sync'); do_settings_sections('press_sync'); ?>
        <table class="form-table">
          <tr><th>RPC URL</th><td><input type="text" name="press_sync_rpc_url" value="<?php echo esc_attr(get_option('press_sync_rpc_url','https://rpc.pressblockchain.io')); ?>" class="regular-text"/></td></tr>
          <tr><th>Outlet ID</th><td><input type="text" name="press_sync_outlet_id" value="<?php echo esc_attr(get_option('press_sync_outlet_id','')); ?>" class="regular-text"/></td></tr>
          <tr><th>Official Outlet Domain</th><td><input type="text" name="press_sync_outlet_domain" value="<?php echo esc_attr(get_option('press_sync_outlet_domain','')); ?>" class="regular-text"/></td></tr>
          <tr><th>PRESS Token Address</th><td><input type="text" name="press_sync_press_token" value="<?php echo esc_attr(get_option('press_sync_press_token','')); ?>" class="regular-text"/></td></tr>
          <tr><th>Treasury Wallet</th><td><input type="text" name="press_sync_treasury_wallet" value="<?php echo esc_attr(get_option('press_sync_treasury_wallet','')); ?>" class="regular-text"/></td></tr>
          <tr><th>Publish Fee (PRESS)</th><td><input type="number" step="1" name="press_sync_publish_fee_press" value="<?php echo esc_attr(get_option('press_sync_publish_fee_press','25')); ?>"/></td></tr>
          <tr><th>Vote Fee (PRESS)</th><td><input type="number" step="1" name="press_sync_vote_fee_press" value="<?php echo esc_attr(get_option('press_sync_vote_fee_press','2')); ?>"/></td></tr>
          <tr><th>Outlet Mode (Press-first admin)</th><td><label><input type="checkbox" name="press_sync_outlet_mode_enabled" value="1" <?php checked(get_option('press_sync_outlet_mode_enabled','1'),'1'); ?> /> Enable (recommended)</label></td></tr>
          <tr><th>Installer API Base URL</th><td><input type="text" name="press_sync_installer_api" value="<?php echo esc_attr(get_option('press_sync_installer_api','http://deploy.pressblockchain.io')); ?>" class="regular-text"/></td></tr>
          <tr><th>Press Oracle AI Endpoint</th><td><input type="text" name="press_sync_ai_endpoint" value="<?php echo esc_attr(get_option('press_sync_ai_endpoint','')); ?>" class="regular-text" placeholder="https://oracle.pressblockchain.io/api/moderate"/></td></tr>
          <tr><th>Tips Protocol Fee (%)</th><td><input type="number" step="0.1" name="press_sync_tip_fee_percent" value="<?php echo esc_attr(get_option('press_sync_tip_fee_percent','1.0')); ?>"/></td></tr>
        </table>
        <?php submit_button('Save Settings'); ?>
      </form>
    </div>
    <?php
  }

  public static function render_dashboard() {
    ?>
    <div class="wrap press-sync">
      <h1>Outlet Dashboard</h1>
      <div class="press-sync-grid">
        <div class="press-card">
          <h3>Publishing Status</h3>
          <div id="press-sync-publish-status" class="press-mono">Waiting…</div>
          <button class="button button-primary" id="press-sync-test-connection">Test Chain Connection</button>
        </div>
        <div class="press-card">
          <h3>Live Votes</h3>
          <div class="press-sub">Articles auto-close voting at 72 hours.</div>
          <div id="press-sync-live-votes"></div>
        </div>
        <div class="press-card">
          <h3>Revenue</h3>
          <div class="press-sub">Treasury receives protocol fees; authors receive 100% article revenue.</div>
          <div id="press-sync-revenue"></div>
        </div>
        <div class="press-card">
          <h3>Co-Authors</h3>
          <div class="press-sub">Add a secondary co-author for a small PRESS fee. Revenue splits 50/50 forever; rights control stays with the primary author.</div>
          <div id="press-sync-coauthors"></div>
        </div>
      </div>
    </div>
    <?php
  }
}
