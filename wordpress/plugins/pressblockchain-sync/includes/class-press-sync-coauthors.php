<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Coauthors {
  public static function init() {
    add_action('add_meta_boxes', [__CLASS__,'meta_box']);
    add_action('save_post', [__CLASS__,'save_meta'], 10, 2);
  }

  public static function meta_box() {
    add_meta_box('press_coauthors', 'Press Co-Author', [__CLASS__,'render_box'], 'post', 'side', 'high');
  }

  public static function render_box($post) {
    $secondary = get_post_meta($post->ID, '_press_secondary_author_wallet', true);
    ?>
    <p>Add a secondary co-author wallet (50/50 revenue split). Primary author retains rights control.</p>
    <input type="text" name="press_secondary_author_wallet" value="<?php echo esc_attr($secondary); ?>" style="width:100%;" placeholder="0x..."/>
    <p class="description">A small PRESS fee applies when adding or changing a secondary co-author.</p>
    <?php
  }

  public static function save_meta($post_id, $post) {
    if (defined('DOING_AUTOSAVE') && DOING_AUTOSAVE) return;
    if ($post->post_type !== 'post') return;
    if (!current_user_can('edit_post', $post_id)) return;
    if (!isset($_POST['press_secondary_author_wallet'])) return;

    $wallet = sanitize_text_field($_POST['press_secondary_author_wallet']);
    $prev = get_post_meta($post_id, '_press_secondary_author_wallet', true);

    if ($wallet && $wallet !== $prev) {
      update_post_meta($post_id, '_press_secondary_author_wallet', $wallet);
      update_post_meta($post_id, '_press_secondary_author_split', '50_50');
      update_post_meta($post_id, '_press_secondary_author_lock', 'true'); // cannot be removed without rights sale event on chain
    }
  }
}
