<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Licensing {
  public static function init() {
    add_shortcode('press_license_widget', [__CLASS__,'shortcode_license']);
  }

  public static function shortcode_license($atts) {
    $postId = get_the_ID();
    $hash = get_post_meta($postId, '_press_provenance_hash', true);
    ob_start(); ?>
      <div class="press-license-widget" data-post="<?php echo esc_attr($postId); ?>">
        <div class="press-license-title">License this Article</div>
        <div class="press-license-sub">Provable origin hash: <span class="press-mono"><?php echo esc_html($hash ?: 'pending'); ?></span></div>
        <div class="press-license-actions">
          <button class="press-btn">Syndicate</button>
          <button class="press-btn">License to University</button>
          <button class="press-btn">License for AI Training</button>
        </div>
      </div>
    <?php return ob_get_clean();
  }
}
