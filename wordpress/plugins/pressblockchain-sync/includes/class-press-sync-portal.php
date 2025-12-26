<?php
if (!defined('ABSPATH')) exit;

class Press_Sync_Portal {
  public static function init() {
    add_shortcode('press_outlet_portal', [__CLASS__,'shortcode']);
    add_action('wp_ajax_press_sync_portal_stats', [__CLASS__,'ajax_stats']);
    add_action('wp_ajax_nopriv_press_sync_portal_stats', [__CLASS__,'ajax_stats']);
  }

  public static function shortcode() {
    ob_start(); ?>
    <div class="press-portal">
      <div class="press-portal-hero">
        <div>
          <div class="press-portal-title">Outlet Portal</div>
          <div class="press-portal-sub">Submit articles, track approvals, manage licensing, and view earnings — powered by Press Blockchain.</div>
        </div>
        <a class="press-btn" href="<?php echo esc_url(site_url('/?press_submit=1')); ?>">Submit Article</a>
      </div>

      <div class="press-portal-grid">
        <div class="press-card">
          <div class="press-card-title">Submission Status</div>
          <div class="press-muted" id="press_portal_status">Loading…</div>
        </div>
        <div class="press-card">
          <div class="press-card-title">Today’s Activity</div>
          <div class="press-muted" id="press_portal_activity">Loading…</div>
        </div>
        <div class="press-card">
          <div class="press-card-title">Earnings (Vault)</div>
          <div class="press-muted">Coming online: tips, licensing, syndication royalties.</div>
        </div>
      </div>

      <div class="press-card" style="margin-top:14px;">
        <div class="press-card-title">Latest Submissions</div>
        <div class="press-muted">Shows the latest on-chain submissions for this outlet (fee-verified + AI-gated).</div>
        <div id="press_portal_list" style="margin-top:10px;">Loading…</div>
      </div>
    </div>
    <?php return ob_get_clean();
  }

  public static function ajax_stats() {
    $pending = new WP_Query(['post_type'=>'post','post_status'=>'pending','posts_per_page'=>1,'meta_key'=>'_press_submit_status','meta_value'=>'pending']);
    $rejected = new WP_Query(['post_type'=>'post','post_status'=>'pending','posts_per_page'=>1,'meta_key'=>'_press_submit_status','meta_value'=>'rejected']);
    $published = new WP_Query(['post_type'=>'post','post_status'=>'publish','posts_per_page'=>1,'meta_key'=>'_press_submit_status','meta_value'=>'published']);

    $latest = new WP_Query(['post_type'=>'post','post_status'=>['pending','publish'],'posts_per_page'=>10,'meta_key'=>'_press_submit_status']);
    $items=[];
    while($latest->have_posts()){ $latest->the_post();
      $pid=get_the_ID();
      $items[]=[
        'id'=>$pid,
        'title'=>get_the_title(),
        'url'=>get_permalink(),
        'status'=>get_post_meta($pid,'_press_submit_status',true),
        'txid'=>get_post_meta($pid,'_press_submit_txid',true),
        'date'=>get_the_date().' '.get_the_time()
      ];
    }
    wp_reset_postdata();

    wp_send_json([
      'ok'=>true,
      'counts'=>[
        'pending'=>$pending->found_posts,
        'rejected'=>$rejected->found_posts,
        'published'=>$published->found_posts
      ],
      'latest'=>$items
    ]);
  }
}
