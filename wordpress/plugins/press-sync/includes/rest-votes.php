<?php
add_action('rest_api_init', function () {
  register_rest_route('press-sync/v1', '/article-votes', array(
    'methods' => 'GET',
    'permission_callback' => '__return_true',
    'callback' => function($req){
      $articleId = sanitize_text_field($req->get_param('articleId'));
      if(!$articleId) return new WP_REST_Response(array('error'=>'missing'), 400);
      $gateway = get_option('press_sync_gateway', 'https://rpc.pressblockchain.io');
      $url = rtrim($gateway,'/').'/articles/votes?articleId='.rawurlencode($articleId);
      $resp = wp_remote_get($url, array('timeout'=>5));
      if(is_wp_error($resp)) return new WP_REST_Response(array('error'=>'gateway_unreachable'), 502);
      $body = wp_remote_retrieve_body($resp);
      $json = json_decode($body, true);
      if(!$json) return new WP_REST_Response(array('error'=>'bad_response'), 502);
      return new WP_REST_Response($json, 200);
    }
  ));
});
