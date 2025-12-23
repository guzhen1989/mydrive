import React from 'react';

export default function ObjectList({ objects = [], onDownload, onUpload }: any) {
  const fileInputRef = React.useRef<HTMLInputElement|null>(null);
  const handleUploadClick = () => fileInputRef.current?.click();
  const onFileChange = (e:React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) onUpload(f);
  };

  return (
    <div>
      <div style={{ marginBottom: 6 }}>
        <button onClick={handleUploadClick}>上传文件到当前 Bucket</button>
        <input ref={fileInputRef} type="file" style={{ display: 'none' }} onChange={onFileChange} />
      </div>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead><tr><th>Name</th><th>Size</th><th>LastModified</th><th>Actions</th></tr></thead>
        <tbody>
          {objects.map((o:any) => (
            <tr key={o.Key}>
              <td style={{ borderBottom: '1px solid #eee' }}>{o.Key}</td>
              <td style={{ borderBottom: '1px solid #eee' }}>{o.Size}</td>
              <td style={{ borderBottom: '1px solid #eee' }}>{new Date(o.LastModified).toLocaleString()}</td>
              <td style={{ borderBottom: '1px solid #eee' }}>
                <button onClick={() => onDownload(o)}>打开 / 下载</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}