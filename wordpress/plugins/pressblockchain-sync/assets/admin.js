jQuery(function($){
  $('#press-sync-test-connection').on('click', async function(){
    const res = await fetch(PressSync.ajax, {
      method:'POST',
      headers:{'Content-Type':'application/x-www-form-urlencoded'},
      body: new URLSearchParams({ action:'press_sync_chain_ping', nonce:PressSync.nonce }).toString()
    });
    const j = await res.json();
    $('#press-sync-publish-status').text(j.ok ? ('Connected: '+j.rpc) : 'Failed');
  });
});
