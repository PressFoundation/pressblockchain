async function pressSyncFetchVotes(articleId){
  const res = await fetch(`/wp-json/press-sync/v1/article-votes?articleId=${encodeURIComponent(articleId)}`, {credentials:'same-origin'});
  if(!res.ok) return null;
  return await res.json();
}
function renderBar(el, data){
  el.querySelectorAll('[data-role]').forEach(node=>{
    const role=node.getAttribute('data-role');
    const r=(data.roles||[]).find(x=>x.role===role);
    if(!r) return;
    node.querySelector('.yes').textContent=r.yes;
    node.querySelector('.no').textContent=r.no;
    node.querySelector('.thr').textContent=r.thrYes;
    const pct=Math.min(100,(r.yes/Math.max(1,r.thrYes))*100);
    node.querySelector('.fill').style.width=pct+'%';
  });
  const left=Math.max(0, data.endTs*1000-Date.now());
  el.querySelector('.time-left').textContent=left>0?Math.ceil(left/60000)+' min':'ended';
}
window.PressSyncVoteBar = async function(articleId){
  const el=document.getElementById('press-sync-vote-bar'); if(!el) return;
  const tick=async()=>{ const d=await pressSyncFetchVotes(articleId); if(d) renderBar(el,d); };
  await tick(); setInterval(tick, 5000);
};
