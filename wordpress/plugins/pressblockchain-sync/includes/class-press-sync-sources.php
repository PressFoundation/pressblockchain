<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Sources {
  public static function init() {
    add_action('admin_menu', [__CLASS__,'menu']);
  }

  public static function menu() {
    add_submenu_page('press-sync', 'Sources', 'Sources', 'manage_options', 'press-sync-sources', [__CLASS__,'render']);
  }

  public static function render() {
    $registry = esc_html(get_option('press_sync_source_registry',''));
    ?>
    <div class="wrap press-sync">
      <h1>Sources</h1>
      <div class="press-sub">
        Sources are KYC-backed roles that can be attached to articles and receive protocol-enforced revenue share.  
        Use this page to configure Source modules and view Source activity for your outlet.
      </div>

      <table class="form-table" style="margin-top:14px;">
        <tr>
          <th>Source Registry Contract</th>
          <td><input type="text" class="regular-text" name="press_sync_source_registry" value="<?php echo $registry; ?>" disabled /></td>
        </tr>
        <tr>
          <th>Source Pool</th>
          <td>
            Explorer-friendly directory powered by events: <code>SourceListed</code>, <code>SourceRegistered</code>, <code>SourceAttached</code>.
          </td>
        </tr>
      </table>

      <div style="margin-top:14px;" class="press-muted">
        Source Earnings: this panel reads indexer-backed snapshots (and falls back safely if offline).
      </div>
    
<h2 style="margin-top:18px;">Source Earnings</h2>
<p class="press-muted">If the indexer is enabled, earnings reconcile to on-chain events and are visible in explorers. This view is a convenience layer.</p>
<div style="border:1px solid #e6eef3; border-radius:14px; padding:12px; background:#fff;">
  <div><strong>Total (PRESS):</strong> <em>Indexer-backed</em></div>
  <div class="press-muted">Enable the Indexer module to display live totals and per-article earnings.</div>
</div>

    </div>
    <?php
  }
}
