import React from 'react';

export default function BucketList({ buckets = [], onSelect }: any) {
  return (
    <div>
      <ul style={{ listStyle: 'none', padding: 0 }}>
        {buckets.map((b:any) => (
          <li key={b.Name} style={{ padding: '6px 4px', borderBottom: '1px solid #eee', cursor: 'pointer' }}
              onClick={() => onSelect(b.Name)}>
            <strong>{b.Name}</strong><div style={{ fontSize: 12, color: '#666' }}>创建于 {b.CreationDate}</div>
          </li>
        ))}
      </ul>
    </div>
  );
}