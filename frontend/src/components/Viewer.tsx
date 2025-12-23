import React, { useEffect, useRef, useState } from 'react';
const { event } = window.__TAURI__.event;

export default function Viewer({ info, onClose }: any) {
  const { type, path, key } = info;
  const [progress, setProgress] = useState<{ written?: number, total?: number }>({});
  useEffect(() => {
    let unlistenProgress: any;
    let unlistenComplete: any;
    (async () => {
      unlistenProgress = await event.listen('stream-progress', (e: any) => {
        const p = e.payload as any;
        if (p.key === key) setProgress({ written: p.written, total: p.total });
      });
      unlistenComplete = await event.listen('stream-complete', (e: any) => {
        const p = e.payload as any;
        if (p.key === key) setProgress((s) => ({ ...s, written: s.total ?? s.written }));
      });
    })();
    return () => {
      unlistenProgress && unlistenProgress();
      unlistenComplete && unlistenComplete();
    };
  }, [key]);

  return (
    <div style={{
      position: 'fixed', left: 0, top: 0, right:0, bottom:0, background:'rgba(0,0,0,0.6)',
      display:'flex', alignItems:'center', justifyContent:'center'
    }}>
      <div style={{ background:'#fff', padding: 12, maxWidth:'90%', maxHeight:'90%' }}>
        <div style={{ textAlign: 'right' }}><button onClick={onClose}>关闭</button></div>
        <div style={{ width: '100%', height: '100%' }}>
          {type === 'image' ? (
            <img src={`file://${path}`} style={{ maxWidth:'100%', maxHeight:'80vh' }} />
          ) : (
            <>
              <video controls style={{ maxWidth:'100%', maxHeight:'80vh' }}>
                <source src={`file://${path}`} />
                你的浏览器不支持 video 标签。
              </video>
              <div style={{ marginTop: 8 }}>下载进度: {progress.written ?? 0}/{progress.total ?? '未知'}</div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}