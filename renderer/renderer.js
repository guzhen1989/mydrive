const logEl = document.getElementById('log');
function log(s){ logEl.textContent += s + '\\n'; logEl.scrollTop = logEl.scrollHeight; }

document.getElementById('btnConnect').addEventListener('click', async () => {
  const cfg = {
    endPoint: document.getElementById('endpoint').value,
    port: document.getElementById('port').value,
    useSSL: document.getElementById('useSSL').checked,
    accessKey: document.getElementById('accessKey').value,
    secretKey: document.getElementById('secretKey').value
  };
  log('尝试连接...');
  try {
    const buckets = await window.api.connect(cfg);
    log('连接成功，Buckets: ' + JSON.stringify(buckets.map(b=>b.name)));
    const ul = document.getElementById('buckets');
    ul.innerHTML = '';
    buckets.forEach(b => {
      const li = document.createElement('li');
      li.textContent = b.name;
      li.style.cursor = 'pointer';
      li.addEventListener('click', () => selectBucket(b.name));
      ul.appendChild(li);
    });
  } catch (err) {
    log('连接失败：' + err);
  }
});

let currentBucket = null;
async function selectBucket(bucket) {
  currentBucket = bucket;
  log('选择 bucket: ' + bucket);
  try {
    const objects = await window.api.listObjects(bucket);
    const area = document.getElementById('objectArea');
    area.innerHTML = '<h4>Objects in ' + bucket + '</h4>';
    const table = document.createElement('table');
    table.border = '1';
    const header = table.insertRow();
    ['Name','Size','LastModified','Actions'].forEach(h => header.insertCell().textContent = h);
    objects.forEach(o => {
      const r = table.insertRow();
      r.insertCell().textContent = o.name;
      r.insertCell().textContent = o.size;
      r.insertCell().textContent = new Date(o.lastModified).toLocaleString();
      const actionCell = r.insertCell();
      const dl = document.createElement('button');
      dl.textContent = '下载';
      dl.addEventListener('click', async () => {
        const local = require('path').join(require('os').homedir(), 'Downloads', o.name);
        try {
          log('开始下载 ' + o.name + ' 到 ' + local);
          await window.api.downloadObject(bucket, o.name, local);
          log('下载完成: ' + local);
        } catch(e) { log('下载出错: ' + e); }
      });
      actionCell.appendChild(dl);
    });
    area.appendChild(table);
  } catch (e) {
    log('列对象失败: ' + e);
  }
}

document.getElementById('btnUpload').addEventListener('click', async () => {
  if (!currentBucket) return log('请先选择 bucket');
  const fileInput = document.getElementById('fileInput');
  if (fileInput.files.length === 0) return log('请选择文件');
  const file = fileInput.files[0];
  // In Electron renderer we don't have direct access to local path for security,
  // so a simple approach is use the File.path (only available if electron's file picker allowed it)
  // For simplicity, try to use file.path (works in Electron)
  if (!file.path) return log('无法获取文件本地路径 (需要 electron file.path 支持)');
  try {
    log('上传 ' + file.name);
    const etag = await window.api.uploadObject(currentBucket, file.name, file.path);
    log('上传成功 etag=' + etag);
    // refresh objects
    selectBucket(currentBucket);
  } catch (e) {
    log('上传失败: ' + e);
  }
});