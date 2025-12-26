<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Chain {
  public static function init() {
    add_action('wp_ajax_press_sync_chain_ping', [__CLASS__,'ajax_ping']);
    add_action('wp_ajax_nopriv_press_sync_chain_ping', [__CLASS__,'ajax_ping']);
    add_action('save_post', [__CLASS__,'on_save_post'], 10, 3);
  }

  public static function ajax_ping() {
    check_ajax_referer('press_sync_nonce','nonce');
    $rpc = get_option('press_sync_rpc_url','');
    wp_send_json(['ok'=> true, 'rpc'=>$rpc, 'time'=> time()]);
  }

  /**
   * Invisible Chain Publishing™
   * - hashes content + metadata
   * - stores provenance locally and queues a chain commit (handled by relay)
   */
  public static function on_save_post($post_ID, $post, $update) {
    if (wp_is_post_revision($post_ID) || $post->post_status !== 'publish') return;
    $content = $post->post_content;
    $title = $post->post_title;
    $author = $post->post_author;
    $hash = hash('sha256', $title.'|'.$content.'|'.$author.'|'.$post->post_date_gmt);
    update_post_meta($post_ID, '_press_provenance_hash', $hash);
    update_post_meta($post_ID, '_press_provenance_ts', time());
    // Queue commit (offchain relay will submit on-chain)
    update_post_meta($post_ID, '_press_chain_commit_status', 'queued');
  }
}
