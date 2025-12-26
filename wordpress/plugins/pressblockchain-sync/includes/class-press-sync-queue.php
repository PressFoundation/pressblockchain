<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Queue {
  public static function init() {
    add_action('admin_menu', [__CLASS__,'menu']);
  }

  public static function menu() {
    add_submenu_page('press-sync', 'Submission Queue', 'Submission Queue', 'manage_options', 'press-sync-queue', [__CLASS__,'render']);
  }

  public static function render() {
    $status = sanitize_text_field($_GET['status'] ?? 'pending');
    $allowed = ['pending','rejected','published'];
    if (!in_array($status,$allowed,true)) $status='pending';

    $meta_key = '_press_submit_status';
    $q = new WP_Query([
      'post_type'=>'post',
      'post_status'=> ($status==='published') ? 'publish' : 'pending',
      'posts_per_page'=>50,
      'meta_query'=>[
        [
          'key'=>$meta_key,
          'value'=>$status,
          'compare'=>'='
        ]
      ]
    ]);

    ?>
    <div class="wrap press-sync">
      <h1>Submission Queue</h1>
      <div class="press-sub">Review pending submissions, AI decisions, and fee receipts. This is designed to replace default WordPress workflows for outlets running on Press Blockchain.</div>
      <div style="margin-top:12px;">
        <a class="button <?php echo $status==='pending'?'button-primary':''; ?>" href="?page=press-sync-queue&status=pending">Pending</a>
        <a class="button <?php echo $status==='rejected'?'button-primary':''; ?>" href="?page=press-sync-queue&status=rejected">Rejected</a>
        <a class="button <?php echo $status==='published'?'button-primary':''; ?>" href="?page=press-sync-queue&status=published">Published</a>
      </div>

      <table class="widefat striped" style="margin-top:14px;">
        <thead><tr>
          <th>Post</th><th>AI</th><th>Fee TXID</th><th>Created</th>
        </tr></thead>
        <tbody>
        <?php if ($q->have_posts()): while ($q->have_posts()): $q->the_post();
          $pid=get_the_ID();
          $ai=get_post_meta($pid,'_press_ai_review',true);
          $reason=get_post_meta($pid,'_press_ai_reason',true);
          $txid=get_post_meta($pid,'_press_submit_txid',true);
          ?>
          <tr>
            <td>
              <strong><a href="<?php echo esc_url(get_edit_post_link($pid)); ?>"><?php echo esc_html(get_the_title()); ?></a></strong>
              <div class="press-muted">ID: <?php echo intval($pid); ?></div>
            </td>
            <td>
              <div><strong><?php echo esc_html($ai ?: 'pending'); ?></strong></div>
              <?php if($reason): ?><div class="press-muted"><?php echo esc_html($reason); ?></div><?php endif; ?>
            </td>
            <td class="press-mono"><?php echo esc_html($txid ?: '—'); ?></td>
            <td><?php echo esc_html(get_the_date().' '.get_the_time()); ?></td>
          </tr>
        <?php endwhile; wp_reset_postdata(); else: ?>
          <tr><td colspan="4">No items.</td></tr>
        <?php endif; ?>
        </tbody>
      </table>
    </div>
    <?php
  }
}
