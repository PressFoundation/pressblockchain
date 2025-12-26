<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Submit {
  public static function init() {
    add_shortcode('press_submit_article', [__CLASS__,'shortcode_submit']);
    add_action('wp_ajax_press_sync_submit', [__CLASS__,'ajax_submit']);
    add_action('wp_ajax_nopriv_press_sync_submit', [__CLASS__,'ajax_submit']);
  }

  public static function shortcode_submit($atts) {
    $publishFee = floatval(get_option('press_sync_publish_fee_press','25'));
    ob_start(); ?>
      <div class="press-submit">
        <div class="press-submit-head">
          <div class="press-submit-title">Submit an Article</div>
          <div class="press-submit-sub">Pay the submission fee, pass safety review, and publish to the outlet with on-chain provenance.</div>
        </div>

        <div class="press-submit-card">
          <label>Title</label>
          <input type="text" id="press_submit_title" placeholder="Headline…" />

          <label>Content</label>
          <textarea id="press_submit_content" rows="10" placeholder="Write your article…"></textarea>

          <label>Optional Image URL</label>
          <input type="text" id="press_submit_image" placeholder="https://…" />

          
<label>Payment TXID (PRESS fee)</label>
<input type="text" id="press_submit_txid" placeholder="0x… (required)" />

          <div class="press-submit-meta" id="press_submit_intent">
            <div class="press-muted" id="press_submit_intent_memo">Preparing payment…</div>
            <div><strong>Submission fee:</strong> <?php echo esc_html($publishFee); ?> PRESS</div>
            <div class="press-muted">The outlet treasury receives protocol fees. Authors receive 100% article revenue.</div>
          </div>

          <div style="display:flex;gap:10px;flex-wrap:wrap;align-items:center;">
  <button class="press-btn" id="press_wallet_connect_btn">Connect PRESS Wallet</button>
  <button class="press-btn" id="press_submit_pay_btn">Pay Fee in PRESS</button>
  <button class="press-btn" id="press_submit_btn">Submit Article</button>
</div>
          <div class="press-submit-note">Connect your wallet, pay the outlet fee in PRESS, and the TXID will auto-fill. Manual TXID entry remains as a fallback.</div>
          <div id="press_submit_status" class="press-muted" style="margin-top:10px;">&nbsp;</div>
        </div>
      </div>
    <?php return ob_get_clean();
  }

  public static function ajax_submit() {
    check_ajax_referer('press_sync_nonce','nonce');

    $title = sanitize_text_field($_POST['title'] ?? '');
    $content = wp_kses_post($_POST['content'] ?? '');
    $image = esc_url_raw($_POST['image'] ?? '');
    $txid = sanitize_text_field($_POST['txid'] ?? '');

    if (!$title || !$content) {
      wp_send_json(['ok'=>false,'error'=>'Title and content required']);
    }

    if (!$txid || strpos($txid,'0x')!==0) {
      wp_send_json(['ok'=>false,'error'=>'Submission fee TXID required']);
    }


// Verify payment TXID against chain via Installer API
$rpc = get_option('press_sync_rpc_url','');
$press_token = get_option('press_sync_press_token','');
$treasury = get_option('press_sync_treasury_wallet','');
$min_press = floatval(get_option('press_sync_publish_fee_press','25'));
$installer = rtrim(get_option('press_sync_installer_api',''),'/');

if ($rpc && $press_token && $treasury && $installer) {
  $verify_payload = json_encode([
    'rpc'=>$rpc,
    'txid'=>$txid,
    'press_token'=>$press_token,
    'treasury'=>$treasury,
    'min_amount_press'=>$min_press
  ]);
  $vresp = wp_remote_post($installer.'/api/fees/verify', [
    'headers'=>['Content-Type'=>'application/json'],
    'body'=>$verify_payload,
    'timeout'=>25
  ]);
  if (!is_wp_error($vresp)) {
    $vb = wp_remote_retrieve_body($vresp);
    $vj = json_decode($vb,true);
    if (!(is_array($vj) && !empty($vj['ok']))) {
      wp_send_json(['ok'=>false,'error'=>'Payment not verified on-chain (check TXID, amount, or treasury)']);
    }
  } else {
    wp_send_json(['ok'=>false,'error'=>'Unable to verify payment at this time']);
  }
} else {
  wp_send_json(['ok'=>false,'error'=>'Outlet SYNC not fully configured (RPC/PRESS/Treasury/Installer API)']);
}


    // Create as pending until AI moderation passes
    $post_id = wp_insert_post([
      'post_title' => $title,
      'post_content' => $content,
      'post_status' => 'pending',
      'post_type' => 'post'
    ]);

    if (is_wp_error($post_id) || !$post_id) {
      wp_send_json(['ok'=>false,'error'=>'Failed to create post']);
    }

    if ($image) update_post_meta($post_id, '_press_submit_image', $image);

    // Record submission intent + fee (actual chain settlement wired next pass)
    update_post_meta($post_id, '_press_submit_fee_press', get_option('press_sync_publish_fee_press','25'));
    update_post_meta($post_id, '_press_submit_txid', $txid);
    update_post_meta($post_id, '_press_submit_status', 'awaiting_ai');

    // AI moderation call (Press Oracle / AI engine). For now, we do a server-side HTTP call to configured endpoint.
    $endpoint = get_option('press_sync_ai_endpoint','');
    $pass = true; $reason = 'OK';

    if ($endpoint) {
      $payload = json_encode([
        'title'=>$title,
        'content'=>$content,
        'image'=>$image,
        'policy'=>[
          'block_porn'=>true,
          'block_graphic'=>true,
          'block_illegal'=>true,
          'block_profanity'=>true,
          'block_illegal_images'=>true
        ]
      ]);

      $resp = wp_remote_post($endpoint, [
        'headers'=>['Content-Type'=>'application/json'],
        'body'=>$payload,
        'timeout'=>20
      ]);

      if (!is_wp_error($resp)) {
        $body = wp_remote_retrieve_body($resp);
        $j = json_decode($body,true);
        if (is_array($j) && isset($j['ok'])) {
          $pass = !!$j['ok'];
          $reason = $j['reason'] ?? ($pass ? 'OK':'Rejected');
        }
      }
    }

    if ($pass) {
      // Promote to publish — treat like any other post; Invisible chain publishing hook will run on publish
      wp_update_post(['ID'=>$post_id,'post_status'=>'publish']);
      update_post_meta($post_id, '_press_submit_status', 'published');
      update_post_meta($post_id, '_press_ai_review', 'approved');
      wp_send_json(['ok'=>true,'postId'=>$post_id,'status'=>'published']);
    } else {
      update_post_meta($post_id, '_press_submit_status', 'rejected');
      update_post_meta($post_id, '_press_ai_review', 'rejected');
      update_post_meta($post_id, '_press_ai_reason', $reason);
      wp_send_json(['ok'=>false,'postId'=>$post_id,'status'=>'rejected','error'=>$reason]);
    }
  }
}
