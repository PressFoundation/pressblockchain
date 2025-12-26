<?php
/**
 * Plugin Name: Press Blockchain SYNC
 * Description: Converts WordPress into a Press Blockchain outlet with invisible on-chain publishing, live vote bars, licensing, tips, and governance tools.
 * Version: 1.0.0
 * Author: Press Labs Inc.
 */

if (!defined('ABSPATH')) exit;

define('PRESS_SYNC_VERSION', '1.0.0');
define('PRESS_SYNC_PATH', plugin_dir_path(__FILE__));
define('PRESS_SYNC_URL', plugin_dir_url(__FILE__));

require_once PRESS_SYNC_PATH . 'includes/class-press-sync-settings.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-chain.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-votes.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-tips.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-coauthors.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-licensing.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-submit.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-queue.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-outlet-mode.php';
require_once PRESS_SYNC_PATH . 'includes/class-press-sync-portal.php';

add_action('admin_menu', function() {
  add_menu_page('Press Blockchain', 'Press Blockchain', 'manage_options', 'press-sync', ['Press_Sync_Settings','render'], 'dashicons-megaphone', 2);
  add_submenu_page('press-sync', 'Outlet Dashboard', 'Outlet Dashboard', 'manage_options', 'press-sync-dashboard', ['Press_Sync_Settings','render_dashboard']);
});

add_action('admin_enqueue_scripts', function($hook){
  if (strpos($hook, 'press-sync') === false) return;
  wp_enqueue_style('press-sync-admin', PRESS_SYNC_URL.'assets/admin.css', [], PRESS_SYNC_VERSION);
  wp_enqueue_script('press-sync-admin', PRESS_SYNC_URL.'assets/admin.js', ['jquery'], PRESS_SYNC_VERSION, true);
  wp_localize_script('press-sync-admin', 'PressSync', [
    'ajax' => admin_url('admin-ajax.php'),
    'nonce' => wp_create_nonce('press_sync_nonce'),
    'rpcUrl' => get_option('press_sync_rpc_url',''),
    'pressToken' => get_option('press_sync_press_token',''),
    'treasury' => get_option('press_sync_treasury_wallet',''),
    'publishFeePress' => floatval(get_option('press_sync_publish_fee_press','25')),
    'chainId' => get_option('press_sync_chain_id',''),
    'explorer' => get_option('press_sync_explorer_url',''),
  ]);
});

add_action('wp_enqueue_scripts', function(){
  wp_enqueue_style('press-sync-front', PRESS_SYNC_URL.'assets/front.css', [], PRESS_SYNC_VERSION);
  wp_enqueue_script('press-sync-front', PRESS_SYNC_URL.'assets/front.js', [], PRESS_SYNC_VERSION, true);
  wp_localize_script('press-sync-front', 'PressSyncFront', [
    'ajax' => admin_url('admin-ajax.php'),
    'nonce' => wp_create_nonce('press_sync_nonce'),
    'rpcUrl' => get_option('press_sync_rpc_url',''),
    'pressToken' => get_option('press_sync_press_token',''),
    'treasury' => get_option('press_sync_treasury_wallet',''),
    'publishFeePress' => floatval(get_option('press_sync_publish_fee_press','25')),
    'chainId' => get_option('press_sync_chain_id',''),
    'explorer' => get_option('press_sync_explorer_url',''),
  ]);
});

Press_Sync_Settings::init();
Press_Sync_Chain::init();
Press_Sync_Votes::init();
Press_Sync_Tips::init();
Press_Sync_Coauthors::init();
Press_Sync_Licensing::init();
Press_Sync_Submit::init();
Press_Sync_Queue::init();
Press_Sync_Outlet_Mode::init();
Press_Sync_Portal::init();
