// 仅展示与之前不同/新增的关键片段（完整文件请替换原 App.tsx）
// 主要改动：上传前使用 list_parts 跳过已上传分片；下载/播放使用 s3_stream_to_temp 并监听事件。

import React, { useEffect, useState } from 'react';
import BucketList from './components/BucketList';
import ObjectList from './components/ObjectList';
import Viewer from './components/Viewer';
import dayjs from 'dayjs';
const { invoke } = window.__TAURI__.tauri;
const { event } = window.__TAURI__.event;

export default function App() {
  // ... 前面保持不变
  useEffect(() => {
    // 监听流事件
    const unlistenProgress = event.listen('stream-progress', (e: any) => {
      const payload = e.payload as any;
      // payload: { key, written, total }
      setLog(l => [...l, `[stream] ${payload.key} written ${payload.written}/${payload.total ?? 'unknown'}`]);
    });
    const unlistenComplete = event.listen('stream-complete', (e: any) => {
      const payload = e.payload as any;
      setLog(l => [...l, `[stream] complete ${payload.key} -> ${payload.path}`]);
    });
    const unlistenError = event.listen('stream-error', (e: any) => {
      const payload = e.payload as any;
      setLog(l => [...l, `[stream] error ${payload.key} : ${payload.error}`]);
    });
    return () => {
      unlistenProgress.then(f => f());
      unlistenComplete.then(f => f());
      unlistenError.then(f => f());
    };
  }, []);

  async function uploadFile(file: File) {
    if (!selectedBucket) { appendLog('请先选择 Bucket'); return; }
    appendLog(`准备上传 ${file.name} (${file.size})`);
    try {
      // start (or resume) multipart upload
      const startRes: any = await invoke('s3_start_multipart_upload', {
        bucket: selectedBucket, key: file.name
      });
      const uploadId = startRes.upload_id;
      appendLog('uploadId: ' + uploadId);
      // query existing parts
      const existing: any = await invoke('s3_list_parts', { bucket: selectedBucket, key: file.name, upload_id: uploadId });
      const uploadedParts: Record<number, string> = {};
      (existing || []).forEach((p: any) => uploadedParts[p.PartNumber] = p.ETag);
      const chunkSize = 5 * 1024 * 1024;
      const parts: { PartNumber: number; ETag: string }[] = [];
      let partNumber = 1;
      for (let offset = 0; offset < file.size; offset += chunkSize) {
        if (uploadedParts[partNumber]) {
          appendLog(`分片 ${partNumber} 已存在，跳过`);
          parts.push({ PartNumber: partNumber, ETag: uploadedParts[partNumber] });
          partNumber++;
          continue;
        }
        const chunk = file.slice(offset, offset + chunkSize);
        const arrayBuffer = await chunk.arrayBuffer();
        const base64 = btoa(String.fromCharCode(...new Uint8Array(arrayBuffer)));
        appendLog(`上传分片 ${partNumber}`);
        const partRes: any = await invoke('s3_upload_part', {
          bucket: selectedBucket, key: file.name, upload_id: uploadId, part_number: partNumber, data_b64: base64
        });
        parts.push({ PartNumber: partNumber, ETag: partRes.etag });
        partNumber++;
      }
      const completeRes: any = await invoke('s3_complete_multipart_upload', {
        bucket: selectedBucket, key: file.name, upload_id: uploadId, parts
      });
      appendLog('上传完成: ' + JSON.stringify(completeRes));
      await listObjects(selectedBucket);
    } catch (e:any) {
      appendLog('上传错误: ' + (e?.message ?? String(e)));
    }
  }

  async function downloadAndView(obj: any) {
    if (!selectedBucket) return;
    appendLog('准备流式下载并打开: ' + obj.Key);
    try {
      const res: any = await invoke('s3_stream_to_temp', { bucket: selectedBucket, key: obj.Key });
      const path = res.path;
      if (obj.Key.match(/\.(png|jpg|jpeg|gif|webp)$/i)) {
        setViewer({ type: 'image', path });
      } else if (obj.Key.match(/\.(mp4|m4v|webm|ogg)$/i)) {
        setViewer({ type: 'video', path, key: obj.Key });
      } else {
        appendLog('已开始下载，路径: ' + path);
      }
    } catch (e:any) {
      appendLog('流式下载失败: ' + (e?.message ?? String(e)));
    }
  }

  // ... rest unchanged
}