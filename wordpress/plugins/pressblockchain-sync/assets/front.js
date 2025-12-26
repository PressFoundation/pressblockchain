(function(){
  async function refresh(bar){
    const postId=bar.getAttribute('data-post');
    const url=PressSyncFront.ajax + '?action=press_sync_votes_get&nonce=' + encodeURIComponent(PressSyncFront.nonce) + '&postId=' + encodeURIComponent(postId);
    const res=await fetch(url);
    const j=await res.json();
    if(!j.ok) return;
    ['journalist','editor','outlet','community'].forEach(r=>{
      const el=bar.querySelector('.count[data-role="'+r+'"]');
      if(el) el.textContent = String(j.counts[r]||0);
    });
    const status=bar.querySelector('.status');
    if(status){
      if(j.open){
        const mins=Math.max(0, Math.floor((j.endsAt*1000-Date.now())/60000));
        status.textContent='Voting open • closes in ' + mins + ' min';
      } else {
        status.textContent='Voting closed';
      }
    }
  }

  function boot(){
    document.querySelectorAll('.press-vote-bar').forEach(bar=>{
      refresh(bar);
      setInterval(()=>refresh(bar), 8000);
    });
  }
  if(document.readyState==='loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();

// Submit Article flow
async function bindSubmit(){
  const connectBtn=document.getElementById('press_wallet_connect_btn');
    const payBtn=document.getElementById('press_submit_pay_btn');
    const btn=document.getElementById('press_submit_btn');
  if(!btn) return;
  btn.addEventListener('click', async ()=>{
    const title=(document.getElementById('press_submit_title')||{}).value||'';
    const content=(document.getElementById('press_submit_content')||{}).value||'';
    const image=(document.getElementById('press_submit_image')||{}).value||'';
        const txid=(document.getElementById('press_submit_txid')||{}).value||'';
    const status=document.getElementById('press_submit_status');
    if(status) status.textContent='Submitting…';
    const body=new URLSearchParams({ action:'press_sync_submit', nonce:PressSyncFront.nonce, title, content, image, txid }).toString();
    const res=await fetch(PressSyncFront.ajax, { method:'POST', headers:{'Content-Type':'application/x-www-form-urlencoded'}, body });
    const j=await res.json();
    if(j.ok){
      if(status) status.textContent='Published. Post ID: '+j.postId;
      window.location.reload();
    } else {
      if(status) status.textContent='Rejected: '+(j.error||'Failed');
    }
  });
}
if(document.readyState==='loading') document.addEventListener('DOMContentLoaded', bindSubmit);
else bindSubmit();


async function loadPaymentIntent(){
  const box=document.getElementById('press_submit_intent');
  if(!box) return;
  const memoEl=document.getElementById('press_submit_intent_memo');
  try{
    // Installer API is configured server-side; we fetch via WP AJAX "chain ping" only for now.
    // Display-only intent: treasury/amount/memo are still governed by SYNC settings.
    if(memoEl) memoEl.textContent='Pay the fee in PRESS to the outlet treasury, then paste the TXID.';
  }catch(e){
    if(memoEl) memoEl.textContent='Payment instructions unavailable.';
  }
}
if(document.readyState==='loading') document.addEventListener('DOMContentLoaded', loadPaymentIntent);
else loadPaymentIntent();


async function pressConnectWallet(){
  const eth=window.ethereum;
  if(!eth) throw new Error("No wallet detected");
  const accounts=await eth.request({method:"eth_requestAccounts"});
  return accounts?.[0] || "";
}

// minimal ABI encoding for transfer(address,uint256)
function encodeTransfer(to, amountWei){
  const sig="a9059cbb"; // keccak transfer(address,uint256)
  const addr=to.toLowerCase().replace("0x","").padStart(64,"0");
  const amt=BigInt(amountWei).toString(16).padStart(64,"0");
  return "0x"+sig+addr+amt;
}

async function pressPayFee(){
  const token=(PressSyncFront.pressToken||"").trim();
  const treasury=(PressSyncFront.treasury||"").trim();
  const fee=Number(PressSyncFront.publishFeePress||25);
  if(!token || !treasury) throw new Error("SYNC not configured (token/treasury)");
  const from=await pressConnectWallet();
  const amountWei=BigInt(Math.floor(fee*1e6))*BigInt(10)**BigInt(12); // fee * 1e18 with 6dp safety
  const data=encodeTransfer(treasury, amountWei.toString());
  const tx=await window.ethereum.request({method:"eth_sendTransaction", params:[{from, to: token, data}]});
  return tx;
}

(function enhanceSubmit(){
  const connectBtn=document.getElementById('press_wallet_connect_btn');
  const payBtn=document.getElementById('press_submit_pay_btn');
  const txInput=document.getElementById('press_submit_txid');

  if(connectBtn){
    connectBtn.addEventListener('click', async ()=>{
      try{
        const a=await pressConnectWallet();
        connectBtn.textContent=`Connected: ${a.slice(0,6)}…${a.slice(-4)}`;
      }catch(e){
        alert(e.message||"Failed to connect wallet");
      }
    });
  }
  if(payBtn){
    payBtn.addEventListener('click', async ()=>{
      try{
        payBtn.disabled=true;
        payBtn.textContent="Paying…";
        const tx=await pressPayFee();
        if(txInput) txInput.value=tx;
        payBtn.textContent="Fee Paid (TX captured)";
      }catch(e){
        alert(e.message||"Payment failed");
        payBtn.textContent="Pay Fee in PRESS";
      }finally{
        payBtn.disabled=false;
      }
    });
  }
})();


(async function portal(){
  const statusEl=document.getElementById('press_portal_status');
  if(!statusEl) return;
  try{
    const res=await fetch(PressSyncFront.ajax, {method:"POST", headers:{"Content-Type":"application/x-www-form-urlencoded"},
      body: new URLSearchParams({action:"press_sync_portal_stats", nonce:PressSyncFront.nonce})});
    const j=await res.json();
    if(!j.ok) throw new Error("failed");
    statusEl.textContent=`Pending: ${j.counts.pending} • Rejected: ${j.counts.rejected} • Published: ${j.counts.published}`;
    const list=document.getElementById('press_portal_list');
    if(list){
      list.innerHTML = (j.latest||[]).map(it=>{
        const tx=it.txid?`<div class="press-muted press-mono">Fee TX: ${it.txid}</div>`:"";
        return `<div style="padding:10px 0;border-bottom:1px solid #eef4f7">
          <div><a href="${it.url}" style="color:#00395E;font-weight:800;text-decoration:none">${it.title}</a></div>
          <div class="press-muted">Status: <strong>${it.status||"—"}</strong> • ${it.date}</div>
          ${tx}
        </div>`;
      }).join('') || '<div class="press-muted">No submissions yet.</div>';
    }
  }catch(e){
    statusEl.textContent="Unable to load portal data.";
  }
})();
