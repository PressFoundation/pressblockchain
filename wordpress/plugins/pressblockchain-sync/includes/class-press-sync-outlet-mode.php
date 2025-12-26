<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Outlet_Mode {
  public static function init() {
    add_action('admin_init', [__CLASS__,'maybe_enable']);
    add_action('admin_menu', [__CLASS__,'prune_menus'], 999);
    add_action('admin_bar_menu', [__CLASS__,'admin_bar'], 999);
  }

  public static function maybe_enable() {
    // Default ON: converts WP admin into a Press outlet cockpit.
    if (get_option('press_sync_outlet_mode_enabled','1') !== '1') return;
  }

  public static function prune_menus() {
    if (get_option('press_sync_outlet_mode_enabled','1') !== '1') return;

    // Keep Posts, Pages, Users, Press Blockchain, and Settings minimal.
    $remove = [
      'edit-comments.php',
      'tools.php',
      'plugins.php',
      'themes.php',
      'options-general.php',
      'upload.php',
    ];
    foreach ($remove as $slug) {
      remove_menu_page($slug);
    }
  }

  public static function admin_bar($wp_admin_bar) {
    if (get_option('press_sync_outlet_mode_enabled','1') !== '1') return;
    $wp_admin_bar->add_node([
      'id'=>'press_sync_submit',
      'title'=>'Submit Article',
      'href'=>admin_url('post-new.php')
    ]);
    $wp_admin_bar->add_node([
      'id'=>'press_sync_queue',
      'title'=>'Submission Queue',
      'href'=>admin_url('admin.php?page=press-sync-queue&status=pending')
    ]);
  }
}
