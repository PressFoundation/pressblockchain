<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Votes {
  public static function init() {
    add_action('wp_ajax_press_sync_votes_get', [__CLASS__,'ajax_get_votes']);
    add_action('wp_ajax_nopriv_press_sync_votes_get', [__CLASS__,'ajax_get_votes']);
    add_shortcode('press_vote_bar', [__CLASS__,'shortcode_vote_bar']);
  }

  public static function ajax_get_votes() {
    check_ajax_referer('press_sync_nonce','nonce');
    $postId = intval($_GET['postId'] ?? 0);
    if (!$postId) wp_send_json(['ok'=>false]);
    $created = intval(get_post_meta($postId, '_press_provenance_ts', true));
    $end = $created + (72*3600);
    $now = time();
    $open = $created > 0 && $now < $end;

    // Placeholder live counts until on-chain indexer is connected
    $counts = [
      'journalist' => intval(get_post_meta($postId, '_press_votes_journalist', true) ?: 0),
      'editor' => intval(get_post_meta($postId, '_press_votes_editor', true) ?: 0),
      'outlet' => intval(get_post_meta($postId, '_press_votes_outlet', true) ?: 0),
      'community' => intval(get_post_meta($postId, '_press_votes_community', true) ?: 0),
    ];

    wp_send_json(['ok'=>true, 'open'=>$open, 'endsAt'=>$end, 'counts'=>$counts]);
  }

  public static function shortcode_vote_bar($atts) {
    $postId = get_the_ID();
    ob_start(); ?>
      <div class="press-vote-bar" data-post="<?php echo esc_attr($postId); ?>">
        <div class="press-vote-row"><span>Journalists</span><span class="count" data-role="journalist">0</span></div>
        <div class="press-vote-row"><span>Editors</span><span class="count" data-role="editor">0</span></div>
        <div class="press-vote-row"><span>Outlets</span><span class="count" data-role="outlet">0</span></div>
        <div class="press-vote-row"><span>Community</span><span class="count" data-role="community">0</span></div>
        <div class="press-vote-time"><span class="status">Loading…</span></div>
      </div>
    <?php return ob_get_clean();
  }
}
