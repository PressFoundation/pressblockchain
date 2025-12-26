<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Tips {
  public static function init() {
    add_shortcode('press_tip_box', [__CLASS__,'shortcode_tip_box']);
  }

  public static function shortcode_tip_box($atts) {
    $postId = get_the_ID();
    $fee = floatval(get_option('press_sync_tip_fee_percent','1.0'));
    ob_start(); ?>
      <div class="press-tip-box" data-post="<?php echo esc_attr($postId); ?>">
        <div class="press-tip-title">Tip the Author</div>
        <div class="press-tip-sub">Supported: BTC, ETH, USDC, PRESS. Protocol fee: <?php echo esc_html($fee); ?>% to treasury.</div>
        <div class="press-tip-actions">
          <button class="press-btn" data-asset="PRESS">Tip PRESS</button>
          <button class="press-btn" data-asset="USDC">Tip USDC</button>
          <button class="press-btn" data-asset="ETH">Tip ETH</button>
          <button class="press-btn" data-asset="BTC">Tip BTC</button>
        </div>
      </div>
    <?php return ob_get_clean();
  }
}
