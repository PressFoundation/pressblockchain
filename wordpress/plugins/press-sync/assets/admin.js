(function(){
  if(!window.PressSync){ return; }

  function qs(id){ return document.getElementById(id); }
  async function post(action, payload){
    const res = await fetch(PressSync.ajax_url, {
      method:'POST',
      headers:{'Content-Type':'application/x-www-form-urlencoded;charset=UTF-8'},
      body: new URLSearchParams({
        action,
        _ajax_nonce: PressSync.nonce,
        payload: JSON.stringify(payload||{})
      }).toString()
    });
    return await res.json();
  }

  // Press Mode toggle
  const t = qs('pressSyncToggleMode');
  if(t){
    t.addEventListener('click', async ()=>{
      const r = await post('press_sync_toggle_mode', {});
      if(r && r.ok){
        document.body.classList.toggle('press-sync-mode', !!r.enabled);
      }
    });
  }

  // Outlet wizard actions
  const btnCreate = qs('pressSyncCreateOutlet');
  if(btnCreate){
    btnCreate.addEventListener('click', async ()=>{
      qs('pressSyncOut').textContent = 'Submitting...';
      const name = qs('ps_outlet_name').value.trim();
      const domain = qs('ps_outlet_domain').value.trim();
      const owner_pk = qs('ps_owner_pk').value.trim();
      const r = await post('press_sync_create_outlet', {name, domain, owner_private_key: owner_pk || undefined});
      qs('pressSyncOut').textContent = JSON.stringify(r, null, 2);
      if(r.ok && r.outlet_id){ qs('ps_outlet_id').value = r.outlet_id; }
    });
  }
  const btnDeploy = qs('pressSyncDeployToken');
  if(btnDeploy){
    btnDeploy.addEventListener('click', async ()=>{
      qs('pressSyncTokOut').textContent = 'Deploying...';
      const domain = qs('ps_outlet_domain').value.trim();
      const token_name = qs('ps_tok_name').value.trim();
      const token_symbol = qs('ps_tok_symbol').value.trim();
      const minted_supply_wei = qs('ps_tok_supply').value.trim();
      const test_transfer_to_self_wei = qs('ps_tok_test').value.trim();
      const owner_pk = qs('ps_owner_pk').value.trim();
      const r = await post('press_sync_deploy_outlet_token', {domain, token_name, token_symbol, minted_supply_wei, test_transfer_to_self_wei, owner_private_key: owner_pk || undefined});
      qs('pressSyncTokOut').textContent = JSON.stringify(r, null, 2);
      if(r.ok && r.token_address){ qs('ps_list_token').value = r.token_address; }
    });
  }
  const btnList = qs('pressSyncListToken');
  if(btnList){
    btnList.addEventListener('click', async ()=>{
      qs('pressSyncListOut').textContent = 'Listing...';
      const domain = qs('ps_outlet_domain').value.trim();
      const token_address = qs('ps_list_token').value.trim();
      const tier = parseInt(qs('ps_list_tier').value,10);
      const owner_pk = qs('ps_owner_pk').value.trim();
      const r = await post('press_sync_list_token', {domain, token_address, tier, owner_private_key: owner_pk || undefined});
      qs('pressSyncListOut').textContent = JSON.stringify(r, null, 2);
    });
  }

  // Quick links: open Press dashboards
  const quick = qs('pressSyncQuickRefresh');
  if(quick){
    quick.addEventListener('click', async ()=>{
      const r = await post('press_sync_refresh_status', {});
      const el = qs('pressSyncStatus');
      if(el){ el.textContent = JSON.stringify(r, null, 2); }
    });
  }
})();
