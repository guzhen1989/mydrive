const { contextBridge, ipcMain } = require('electron');
const Minio = require('minio');
const fs = require('fs');
const path = require('path');

// We will keep a client per connection config
let minioClient = null;

contextBridge.exposeInMainWorld('api', {
  connect: (config) => {
    try {
      minioClient = new Minio.Client({
        endPoint: config.endPoint,
        port: parseInt(config.port || 9000, 10),
        useSSL: !!config.useSSL,
        accessKey: config.accessKey,
        secretKey: config.secretKey
      });
      // quick test: listBuckets to verify connection
      return new Promise((resolve, reject) => {
        minioClient.listBuckets((err, buckets) => {
          if (err) return reject(err.message || String(err));
          resolve(buckets);
        });
      });
    } catch (err) {
      return Promise.reject(err.message || String(err));
    }
  },

  listBuckets: () => {
    if (!minioClient) return Promise.reject('not connected');
    return new Promise((resolve, reject) => {
      minioClient.listBuckets((err, buckets) => {
        if (err) return reject(err.message || String(err));
        resolve(buckets);
      });
    });
  },

  listObjects: (bucket) => {
    if (!minioClient) return Promise.reject('not connected');
    return new Promise((resolve, reject) => {
      const objects = [];
      const stream = minioClient.listObjectsV2(bucket, '', true);
      stream.on('data', obj => objects.push(obj));
      stream.on('error', err => reject(err.message || String(err)));
      stream.on('end', () => resolve(objects));
    });
  },

  downloadObject: (bucket, objectName, localPath) => {
    if (!minioClient) return Promise.reject('not connected');
    return new Promise((resolve, reject) => {
      minioClient.getObject(bucket, objectName, (err, stream) => {
        if (err) return reject(err.message || String(err));
        const writeStream = fs.createWriteStream(localPath);
        stream.pipe(writeStream);
        stream.on('end', () => resolve(localPath));
        stream.on('error', e => reject(e.message || String(e)));
      });
    });
  },

  uploadObject: (bucket, objectName, localFilePath) => {
    if (!minioClient) return Promise.reject('not connected');
    return new Promise((resolve, reject) => {
      minioClient.fPutObject(bucket, objectName, localFilePath, {}, (err, etag) => {
        if (err) return reject(err.message || String(err));
        resolve(etag);
      });
    });
  }
});